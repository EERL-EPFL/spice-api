/// High-performance streaming ZIP with concurrent S3 downloads and immediate streaming
use axum::{
    body::Body,
    http::{
        StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    },
    response::Response,
};
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::sync::mpsc;
const MAX_CONCURRENT: usize = 25;

pub async fn create_hybrid_streaming_zip_response(
    assets: Vec<super::models::Model>,
    config: &crate::config::Config,
) -> Result<Response, (StatusCode, String)> {
    use crate::external::s3::get_client;

    if assets.is_empty() {
        return Err((StatusCode::NOT_FOUND, "No assets to download".to_string()));
    }

    let s3_client = get_client(config).await;
    let (tx, mut rx) = mpsc::channel::<Result<Vec<u8>, std::io::Error>>(32);

    // Clone data for background processing
    let assets_clone = assets.clone();
    let s3_client_clone = s3_client.clone();
    let config_clone = config.clone();

    // Background task: concurrent downloads with controlled concurrency
    tokio::spawn(async move {
        let mut download_futures = FuturesUnordered::new();
        let mut central_directory = Vec::new();
        let mut current_offset: u32 = 0;

        // Process in batches to control memory usage and concurrency
        let chunks: Vec<_> = assets_clone.chunks(MAX_CONCURRENT).collect();

        for batch in &chunks {
            // Start downloads for this batch
            for (file_index, asset) in batch.iter().enumerate() {
                let s3_client = s3_client_clone.clone();
                let bucket = config_clone.s3_bucket_id.clone();
                let key = asset.s3_key.clone();
                let filename = asset.original_filename.clone();

                let download_future = async move {
                    match s3_client
                        .get_object()
                        .bucket(&bucket)
                        .key(&key)
                        .send()
                        .await
                    {
                        Ok(response) => match response.body.collect().await {
                            Ok(data) => {
                                let file_data = data.into_bytes().to_vec();
                                let crc = crc32fast::hash(&file_data);
                                Some((file_index, filename, file_data, crc))
                            }
                            Err(_) => None,
                        },
                        Err(_) => None,
                    }
                };

                download_futures.push(download_future);
            }

            // Wait for batch to complete and stream files in order
            let mut batch_results = Vec::new();
            while let Some(result) = download_futures.next().await {
                if let Some(file_result) = result {
                    batch_results.push(file_result);
                }
            }

            // Sort by index to maintain file order
            batch_results.sort_by_key(|(index, _, _, _)| *index);

            // Stream this batch's files immediately
            for (_, filename, file_data, crc) in batch_results {
                let filename_bytes = filename.as_bytes();
                let file_len = file_data.len() as u32;

                // Build and stream local file header
                let mut local_header = Vec::with_capacity(30 + filename_bytes.len());
                local_header.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]); // Local file header signature
                local_header.extend_from_slice(&[0x14, 0x00]); // Version needed to extract (2.0)
                local_header.extend_from_slice(&[0x00, 0x00]); // General purpose bit flag
                local_header.extend_from_slice(&[0x00, 0x00]); // Compression method (stored)
                local_header.extend_from_slice(&[0x00, 0x00]); // File last modification time
                local_header.extend_from_slice(&[0x00, 0x00]); // File last modification date
                local_header.extend_from_slice(&crc.to_le_bytes()); // CRC-32
                local_header.extend_from_slice(&file_len.to_le_bytes()); // Compressed size
                local_header.extend_from_slice(&file_len.to_le_bytes()); // Uncompressed size
                local_header.extend_from_slice(&(filename_bytes.len() as u16).to_le_bytes()); // File name length
                local_header.extend_from_slice(&[0x00, 0x00]); // Extra field length
                local_header.extend_from_slice(filename_bytes); // File name

                if tx.send(Ok(local_header)).await.is_err() {
                    return;
                }

                // Stream file data in chunks
                const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks
                for chunk in file_data.chunks(CHUNK_SIZE) {
                    if tx.send(Ok(chunk.to_vec())).await.is_err() {
                        return;
                    }
                }

                // Build central directory entry
                let mut cd_entry = Vec::with_capacity(46 + filename_bytes.len());
                cd_entry.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]); // Central directory file header signature
                cd_entry.extend_from_slice(&[0x14, 0x00]); // Version made by
                cd_entry.extend_from_slice(&[0x14, 0x00]); // Version needed to extract
                cd_entry.extend_from_slice(&[0x00, 0x00]); // General purpose bit flag
                cd_entry.extend_from_slice(&[0x00, 0x00]); // Compression method
                cd_entry.extend_from_slice(&[0x00, 0x00]); // Last mod file time
                cd_entry.extend_from_slice(&[0x00, 0x00]); // Last mod file date
                cd_entry.extend_from_slice(&crc.to_le_bytes()); // CRC-32
                cd_entry.extend_from_slice(&file_len.to_le_bytes()); // Compressed size
                cd_entry.extend_from_slice(&file_len.to_le_bytes()); // Uncompressed size
                cd_entry.extend_from_slice(&(filename_bytes.len() as u16).to_le_bytes()); // File name length
                cd_entry.extend_from_slice(&[0x00, 0x00]); // Extra field length
                cd_entry.extend_from_slice(&[0x00, 0x00]); // File comment length
                cd_entry.extend_from_slice(&[0x00, 0x00]); // Disk number start
                cd_entry.extend_from_slice(&[0x00, 0x00]); // Internal file attributes
                cd_entry.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // External file attributes
                cd_entry.extend_from_slice(&current_offset.to_le_bytes()); // Relative offset of local header
                cd_entry.extend_from_slice(filename_bytes); // File name

                central_directory.extend_from_slice(&cd_entry);
                current_offset += 30 + filename_bytes.len() as u32 + file_len;
            }
        }

        // Stream central directory and end record
        let cd_len = central_directory.len() as u32;
        let total_files = assets_clone.len();

        if !central_directory.is_empty() && tx.send(Ok(central_directory)).await.is_err() {
            return;
        }

        let mut end_record = Vec::with_capacity(22);
        end_record.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]); // End of central dir signature
        end_record.extend_from_slice(&[0x00, 0x00]); // Number of this disk
        end_record.extend_from_slice(&[0x00, 0x00]); // Number of disk with start of central directory
        end_record.extend_from_slice(&(total_files as u16).to_le_bytes()); // Total entries this disk
        end_record.extend_from_slice(&(total_files as u16).to_le_bytes()); // Total entries
        end_record.extend_from_slice(&cd_len.to_le_bytes()); // Size of central directory
        end_record.extend_from_slice(&current_offset.to_le_bytes()); // Offset of start of central directory
        end_record.extend_from_slice(&[0x00, 0x00]); // ZIP file comment length

        let _ = tx.send(Ok(end_record)).await;
    });

    let stream = async_stream::stream! {
        while let Some(chunk) = rx.recv().await {
            yield chunk;
        }
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/zip")
        .header(
            CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"bulk-assets-{}.zip\"",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            ),
        )
        .header("Transfer-Encoding", "chunked")
        .header("X-Accel-Buffering", "no")
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .header("Expires", "0")
        .body(Body::from_stream(stream))
        .unwrap())
}
