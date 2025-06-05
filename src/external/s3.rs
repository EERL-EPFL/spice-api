use crate::config::Config;
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::{Client as S3Client, config::Region};
use axum::http::StatusCode;
use futures::stream::{self, StreamExt};
use spice_entity::s3_assets::Model as S3Assets;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

pub async fn get_client(config: &Config) -> Arc<S3Client> {
    let region = Region::new("us-east-1");
    let credentials = Credentials::new(
        &config.s3_access_key,
        &config.s3_secret_key,
        None,
        None,
        "manual",
    );
    let shared_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region.clone())
        .credentials_provider(credentials)
        .endpoint_url(&config.s3_url)
        .load()
        .await;

    Arc::new(S3Client::new(&shared_config))
}

pub async fn delete_from_s3(s3_key: &str) -> Result<(), String> {
    let config = Config::from_env();
    let client = get_client(&config).await;
    let bucket = &config.s3_bucket_id;

    match client
        .delete_object()
        .bucket(bucket)
        .key(s3_key)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Failed to delete object from S3: {err}")),
    }
}

// New function: concurrently download assets from S3 with progress logging.
// Returns the TempDir (to keep files alive) and a vector of (original filename, file path).
pub async fn download_assets(
    assets: Vec<S3Assets>,
    config: &Config,
    s3_client: Arc<S3Client>,
) -> Result<(TempDir, Vec<(String, PathBuf)>), (StatusCode, String)> {
    let temp_dir =
        tempfile::tempdir().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let temp_path = temp_dir.path().to_owned();

    let download_futures = stream::iter(assets.into_iter().map(|asset| {
        let temp_dir_path = temp_path.clone();
        let s3_client = s3_client.clone();
        async move {
            let tmp_file_path = temp_dir_path.join(&asset.original_filename);
            let mut tmp_file = tokio::fs::File::create(&tmp_file_path)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let s3_response = s3_client
                .get_object()
                .bucket(&config.s3_bucket_id)
                .key(&asset.s3_key)
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("S3 download error for {}: {}", asset.s3_key, e),
                    )
                })?;
            let mut body_stream = s3_response.body.into_async_read();
            tokio::io::copy(&mut body_stream, &mut tmp_file)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            Ok((asset.original_filename, tmp_file_path))
        }
    }))
    .buffer_unordered(10) // Adjust concurrency as needed.
    .collect::<Vec<Result<(String, PathBuf), (StatusCode, String)>>>()
    .await;

    let mut asset_paths = Vec::new();
    for result in download_futures {
        match result {
            Ok(tuple) => asset_paths.push(tuple),
            Err(e) => return Err(e),
        }
    }
    Ok((temp_dir, asset_paths))
}
