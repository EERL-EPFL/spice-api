pub use super::models::{Experiment, router as crudrouter};
use crate::common::auth::Role;
use crate::common::state::AppState;
use crate::external::s3::get_client;
use crate::routes::assets::models as s3_assets;
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{extract::Multipart, http::{status::StatusCode, HeaderMap}, response::{Json, Response}, routing::get, Router};
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::CRUDResource;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use serde::Serialize;
use std::convert::TryInto;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

#[cfg(test)]
mod asset_role_tests {
    use super::determine_asset_role;

    #[test]
    fn test_camera_image_detection() {
        // Test camera image patterns
        assert_eq!(
            determine_asset_role("INP_49640_2025-03-20_15-14-17.jpg", "image", "jpg"),
            "camera_image"
        );
        assert_eq!(
            determine_asset_role("INP_12345_2024-12-01_10-30-45.png", "image", "png"),
            "camera_image"
        );
        // Non-camera image
        assert_eq!(
            determine_asset_role("random_photo.jpg", "image", "jpg"),
            "other_image"
        );
    }

    #[test]
    fn test_tabular_data_roles() {
        // Analysis data files
        assert_eq!(
            determine_asset_role("merged.xlsx", "tabular", "xlsx"),
            "analysis_data"
        );
        assert_eq!(
            determine_asset_role("freezing_temperatures.xlsx", "tabular", "xlsx"),
            "analysis_data"
        );
        assert_eq!(
            determine_asset_role("merged.csv", "tabular", "csv"),
            "analysis_data"
        );
        // Temperature sensor data
        assert_eq!(
            determine_asset_role("S39031 INP Freezing.xlsx", "tabular", "xlsx"),
            "temperature_data"
        );
        // Configuration files
        assert_eq!(
            determine_asset_role("regions.yaml", "tabular", "yaml"),
            "configuration"
        );
        assert_eq!(
            determine_asset_role("temperature_config.yaml", "tabular", "yaml"),
            "configuration"
        );
        // Other data
        assert_eq!(
            determine_asset_role("random_data.xlsx", "tabular", "xlsx"),
            "raw_data"
        );
    }

    #[test]
    fn test_other_file_types() {
        // NetCDF analysis files
        assert_eq!(
            determine_asset_role("analysis.nc", "netcdf", "nc"),
            "analysis_data"
        );
        assert_eq!(
            determine_asset_role("well_temperatures.nc", "netcdf", "nc"),
            "analysis_data"
        );
        // Analysis images
        assert_eq!(
            determine_asset_role("analysis_tray_1.png", "image", "png"),
            "analysis_data"
        );
        assert_eq!(
            determine_asset_role("frozen_fraction.png", "image", "png"),
            "analysis_data"
        );
        // Other files
        assert_eq!(
            determine_asset_role("setup.yaml", "unknown", "yaml"),
            "configuration"
        );
        assert_eq!(
            determine_asset_role("random.pdf", "unknown", "pdf"),
            "miscellaneous"
        );
    }
}

/// Determine asset role based on filename patterns and file type
fn determine_asset_role(filename: &str, file_type: &str, _extension: &str) -> String {
    let filename_lower = filename.to_lowercase();
    
    match file_type {
        "image" => {
            // Camera images from INP system follow pattern: INP_XXXXX_YYYY-MM-DD_HH-MM-SS
            if filename_lower.starts_with("inp_") && (filename_lower.contains("2024") || 
               filename_lower.contains("2025") || filename_lower.contains("2026")) {
                "camera_image".to_string()
            } else if filename_lower.contains("analysis") || filename_lower.contains("frozen_fraction") || 
                      filename_lower.contains("regions") || filename_lower.contains("trays_config") {
                "analysis_data".to_string()
            } else {
                "other_image".to_string()
            }
        },
        "tabular" => {
            if filename_lower.contains("freezing_temperatures") || 
               (filename_lower.contains("merged") && (filename_lower.contains("csv") || filename_lower.contains("xlsx"))) ||
               filename_lower.contains("analysis") {
                "analysis_data".to_string()
            } else if filename_lower.contains("inp") && filename_lower.contains("freezing") {
                "temperature_data".to_string() 
            } else if filename_lower.contains("config") || filename_lower.contains("setup") || filename_lower.contains("yaml") {
                "configuration".to_string()
            } else {
                "raw_data".to_string()
            }
        },
        "netcdf" => {
            if filename_lower.contains("analysis") || filename_lower.contains("well_temperatures") {
                "analysis_data".to_string()
            } else {
                "scientific_data".to_string()
            }
        },
        _ => {
            if filename_lower.contains("readme") || filename_lower.contains("doc") {
                "documentation".to_string()
            } else if filename_lower.contains("config") || filename_lower.contains("yaml") || filename_lower.contains("yml") {
                "configuration".to_string()
            } else {
                "miscellaneous".to_string()
            }
        }
    }
}

pub fn router(state: &AppState) -> OpenApiRouter
where
    Experiment: CRUDResource,
{
    let mut mutating_router = crudrouter(&state.db.clone());
    // Excel upload endpoint is handled separately in excel_upload_router()
    // Asset upload/download endpoints are handled separately in asset_router()

    if let Some(instance) = &state.keycloak_auth_instance {
        mutating_router = mutating_router.layer(
            KeycloakAuthLayer::<Role>::builder()
                .instance(instance.clone())
                .passthrough_mode(PassthroughMode::Block)
                .persist_raw_claims(false)
                .expected_audiences(vec![String::from("account")])
                .required_roles(vec![Role::Administrator])
                .build(),
        );
    } else if !state.config.tests_running {
        println!(
            "Warning: Mutating routes of {} router are not protected",
            Experiment::RESOURCE_NAME_PLURAL
        );
    }

    mutating_router
}

/// Separate router for Excel upload endpoint (uses regular Axum Router, not `OpenApiRouter`)
pub fn excel_upload_router() -> Router<AppState> {
    use axum::extract::DefaultBodyLimit;
    
    Router::new()
        .route(
            "/{experiment_id}/process-excel",
            post(super::excel_upload::process_excel_upload),
        )
        .route(
            "/{experiment_id}/process-asset",
            post(process_asset_data),
        )
        .route(
            "/{experiment_id}/clear-results",
            post(clear_experiment_results),
        )
        .layer(DefaultBodyLimit::max(30 * 1024 * 1024)) // 30MB limit to match main router
}

/// Separate router for asset upload/download endpoints (uses regular Axum Router, not `OpenApiRouter`)
pub fn asset_router() -> Router<AppState> {
    use axum::extract::DefaultBodyLimit;
    
    Router::new()
        .route("/{experiment_id}/uploads", post(upload_file))
        .route("/{experiment_id}/download", get(download_experiment_assets))
        .route("/{experiment_id}/download-token", post(create_experiment_download_token))
        .layer(DefaultBodyLimit::max(30 * 1024 * 1024)) // 30MB limit for file uploads
}

#[derive(Serialize, ToSchema)]
pub struct UploadResponse {
    success: bool,
    filename: String,
    size: u64,
    auto_processed: bool,
    processing_message: Option<String>,
}

#[utoipa::path(
    post,
    path = "/{experiment_id}/uploads",
    request_body(
        content_type = "multipart/form-data",
        description = "File to upload",
        example = json!({
            "file": "(binary data)"
        })
    ),
    responses(
        (status = 200, description = "Success", body = UploadResponse)
    )
)]
pub async fn upload_file(
    State(state): State<AppState>,
    Path(experiment_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    mut infile: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    // Check if the experiment exists
    if super::models::Entity::find_by_id(experiment_id)
        .one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_none()
    {
        return Err((StatusCode::NOT_FOUND, "Experiment not found".to_string()));
    }

    // Load S3 configuration from app state (not needed for mocked S3 operations)
    let _s3_client = get_client(&state.config).await;

    while let Some(mut field) = infile.next_field().await.unwrap() {
        let field_name = field.name().unwrap_or("none").to_string();

        // Process only the field named "file"
        if field_name != "file" {
            continue;
        }

        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let extension = std::path::Path::new(&file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        let file_type = match extension.as_str() {
            "png" | "jpg" | "jpeg" => "image".to_string(),
            "xls" | "ods" | "xlsx" | "csv" => "tabular".to_string(),
            "nc" => "netcdf".to_string(),
            _ => "unknown".to_string(),
        };

        let mut file_bytes = Vec::new();
        while let Some(chunk) = field.chunk().await.unwrap() {
            file_bytes.extend_from_slice(&chunk);
        }
        let size = file_bytes.len() as u64;

        // Generate a unique S3 key
        let s3_key = format!(
            "{}/{}/experiments/{}/{}",
            state.config.app_name, state.config.deployment, experiment_id, file_name
        );

        // Check if overwrite is allowed
        let allow_overwrite = headers.get("x-allow-overwrite")
            .and_then(|v| v.to_str().ok())
            .map(|s| s == "true")
            .unwrap_or(false);

        // Check if file already exists in database
        let existing_asset = s3_assets::Entity::find()
            .filter(s3_assets::Column::ExperimentId.eq(Some(experiment_id)))
            .filter(s3_assets::Column::OriginalFilename.eq(&file_name))
            .one(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(existing) = existing_asset {
            if !allow_overwrite {
                return Err((
                    StatusCode::CONFLICT,
                    format!("File '{file_name}' already exists in this experiment"),
                ));
            }
            
            // Delete existing asset from database and S3 before uploading new one
            // Delete from S3 first (uses mock for tests, real S3 for production)
            if let Err(e) = crate::external::s3::delete_from_s3(&existing.s3_key).await {
                println!("Warning: Failed to delete existing file from S3: {} - {}", existing.s3_key, e);
            }

            // Delete from database
            s3_assets::Entity::delete_by_id(existing.id)
                .exec(&state.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete existing asset: {e}")))?;
        }

        // Upload the file to S3 (uses mock for tests, real S3 for production)
        if let Err(e) = crate::external::s3::put_object_to_s3(&s3_key, file_bytes.clone(), &state.config).await {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to upload to S3: {}", e),
            ));
        }

        // Determine asset role based on filename patterns and type
        let asset_role = determine_asset_role(&file_name, &file_type, &extension);

        // Insert a record into the local DB
        let asset = s3_assets::ActiveModel {
            original_filename: Set(file_name.clone()),
            experiment_id: Set(Some(experiment_id)),
            s3_key: Set(s3_key.clone()),
            size_bytes: Set(Some(size.try_into().unwrap())),
            uploaded_by: Set(Some("uploader".to_string())),
            r#type: Set(file_type.clone()),
            role: Set(Some(asset_role.clone())),
            processing_status: Set(None),
            processing_message: Set(None),
            ..Default::default()
        };
        let asset_result = s3_assets::Entity::insert(asset)
            .exec(&state.db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to insert asset record: {e}"),
                )
            })?;

        let asset_id = asset_result.last_insert_id;

        // Auto-process Excel files with analysis_data role (like merged.xlsx)
        let should_auto_process = file_type == "tabular" && 
                                  (extension == "xlsx" || extension == "xls") && 
                                  asset_role == "analysis_data";
        
        let (auto_processed, processing_message) = if should_auto_process {
            println!("🔄 Auto-processing Excel file: {}", file_name);
            
            // Create processing service and trigger processing
            let processing_service = crate::services::data_processing_service::DataProcessingService::new(state.db.clone());
            
            match processing_service.process_excel_file(experiment_id, file_bytes.clone()).await {
                Ok(result) => {
                    // Update asset with processing results
                    let processing_status = match result.status {
                        crate::services::models::ProcessingStatus::Completed => Some("completed".to_string()),
                        crate::services::models::ProcessingStatus::Failed => Some("error".to_string()),
                        _ => Some("processing".to_string()),
                    };
                    
                    let message = if result.status == crate::services::models::ProcessingStatus::Completed {
                        Some(format!("✅ Processed {} temperature readings in {}ms", 
                                   result.temperature_readings_created, result.processing_time_ms))
                    } else if let Some(error) = result.error {
                        Some(format!("❌ Processing failed: {}", error))
                    } else {
                        Some("Processing completed".to_string())
                    };

                    // Update the asset record with processing status
                    if let Ok(_) = crate::routes::assets::models::Entity::update_many()
                        .col_expr(crate::routes::assets::models::Column::ProcessingStatus, 
                                 sea_orm::sea_query::Expr::value(processing_status.clone()))
                        .col_expr(crate::routes::assets::models::Column::ProcessingMessage,
                                 sea_orm::sea_query::Expr::value(message.clone()))
                        .filter(crate::routes::assets::models::Column::Id.eq(asset_id))
                        .exec(&state.db)
                        .await 
                    {
                        println!("✅ Updated asset {} with processing status: {:?}", asset_id, processing_status);
                    }
                    
                    (true, message)
                },
                Err(e) => {
                    let error_msg = format!("❌ Auto-processing failed: {}", e);
                    println!("{}", error_msg);
                    
                    // Update asset with error status
                    let _ = crate::routes::assets::models::Entity::update_many()
                        .col_expr(crate::routes::assets::models::Column::ProcessingStatus, 
                                 sea_orm::sea_query::Expr::value(Some("error".to_string())))
                        .col_expr(crate::routes::assets::models::Column::ProcessingMessage,
                                 sea_orm::sea_query::Expr::value(Some(error_msg.clone())))
                        .filter(crate::routes::assets::models::Column::Id.eq(asset_id))
                        .exec(&state.db)
                        .await;
                    
                    (false, Some(error_msg))
                }
            }
        } else {
            (false, None)
        };

        return Ok(Json(UploadResponse {
            success: true,
            filename: file_name,
            size,
            auto_processed,
            processing_message,
        }));
    }

    Err((StatusCode::BAD_REQUEST, "No file uploaded".to_string()))
}

/// Create a download token for experiment assets
pub async fn create_experiment_download_token(
    State(state): State<AppState>,
    Path(experiment_id): Path<uuid::Uuid>,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, String)> {
    // Verify experiment exists
    use crate::routes::experiments::models::Entity as ExperimentEntity;
    
    let experiment = ExperimentEntity::find_by_id(experiment_id)
        .one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    
    if experiment.is_none() {
        return Err((StatusCode::NOT_FOUND, "Experiment not found".to_string()));
    }
    
    let token = state.create_experiment_download_token(experiment_id).await;
    
    Ok(axum::Json(serde_json::json!({
        "token": token,
        "download_url": format!("/api/assets/download/{}", token)
    })))
}

#[utoipa::path(
    get,
    path = "/{experiment_id}/download",
    responses(
        (status = 200, description = "Zip file of experiment assets", body = Vec<u8>),
        (status = 404, description = "No assets found", body = String),
        (status = 500, description = "Internal Server Error", body = String)
    ),
    operation_id = "download_experiment_assets",
    summary = "Download experiment assets as a zip file",
    description = "Fetches all assets for the given experiment, concurrently downloads them from S3, writes them to temporary files, creates a zip archive on disk, and streams the zip file. For large files or production workloads, consider using a temporary token with a presigned URL."
)]
pub async fn download_experiment_assets(
    State(state): State<AppState>,
    Path(experiment_id): Path<uuid::Uuid>,
) -> Result<Response, (StatusCode, String)> {
    // Query assets for the experiment.
    // let db  = state.db.clone();
    let assets = s3_assets::Entity::find()
        .filter(s3_assets::Column::ExperimentId.eq(Some(experiment_id)))
        .all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if assets.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            "No assets found for this experiment".to_string(),
        ));
    }

    // Use hybrid streaming: concurrent downloads + immediate streaming
    use crate::routes::assets::views::streaming_hybrid;
    streaming_hybrid::create_hybrid_streaming_zip_response(assets, &state.config).await
        .map(|mut response| {
            // Update filename for experiment
            let headers = response.headers_mut();
            headers.insert(
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"experiment_{}.zip\"", experiment_id)
                    .parse()
                    .unwrap()
            );
            response
        })
}

/// Create a true streaming ZIP response for experiment downloads
async fn streaming_experiment_zip_response(
    assets: Vec<s3_assets::Model>,
    config: &crate::config::Config,
    experiment_id: uuid::Uuid,
) -> Result<Response, (StatusCode, String)> {
    use axum::body::Body;
    use tokio::sync::mpsc;
    
    // Calculate exact ZIP size for Content-Length header
    let mut total_zip_size: u64 = 0;
    for asset in &assets {
        let filename_len = asset.original_filename.len() as u64;
        let file_size = asset.size_bytes.unwrap_or(0) as u64;
        
        // Local file header + file data
        total_zip_size += 30 + filename_len + file_size;
        // Central directory entry
        total_zip_size += 46 + filename_len;
    }
    // End of central directory
    total_zip_size += 22;
    
    let s3_client = get_client(config).await;
    
    // Channel for true streaming - send data immediately as it arrives from S3
    let (tx, mut rx) = mpsc::channel::<Result<Vec<u8>, std::io::Error>>(1);
    
    // Clone data for the background task
    let assets_for_task = assets.clone();
    let s3_client_for_task = s3_client.clone();
    let config_for_task = config.clone();
    
    // Spawn task that streams ZIP data immediately as S3 data arrives
    tokio::spawn(async move {
        // Track central directory entries
        let mut central_directory = Vec::new();
        let mut current_offset: u32 = 0;
        
        // Use concurrent downloads for better performance
        use futures::stream::{StreamExt, FuturesUnordered};
        
        let mut download_futures = FuturesUnordered::new();
        
        // Start concurrent downloads (limit to 4 concurrent to avoid overwhelming S3)
        for (file_index, asset) in assets_for_task.iter().enumerate() {
            let s3_client = s3_client_for_task.clone();
            let bucket = config_for_task.s3_bucket_id.clone();
            let key = asset.s3_key.clone();
            let asset_clone = asset.clone();
            
            let download_future = async move {
                let s3_result = s3_client
                    .get_object()
                    .bucket(&bucket)
                    .key(&key)
                    .send()
                    .await;
                
                let (file_data, file_len, crc) = match s3_result {
                    Ok(response) => {
                        match response.body.collect().await {
                            Ok(data) => {
                                let bytes = data.into_bytes().to_vec();
                                let len = bytes.len() as u32;
                                let crc = crc32fast::hash(&bytes);
                                (bytes, len, crc)
                            }
                            Err(_) => return None,
                        }
                    }
                    Err(_) => return None,
                };
                
                Some((file_index, asset_clone, file_data, file_len, crc))
            };
            
            download_futures.push(download_future);
        }
        
        // Collect results and send in order
        let mut results = Vec::with_capacity(assets_for_task.len());
        while let Some(result) = download_futures.next().await {
            if let Some(data) = result {
                results.push(data);
            }
        }
        
        // Sort by file index to maintain order
        results.sort_by_key(|(index, _, _, _, _)| *index);
        
        // Process results in order
        for (file_index, asset, file_data, file_len, crc) in results {
            let filename = asset.original_filename.as_bytes();
            let is_first = file_index == 0;
            
            // Build local file header
            let mut local_header = Vec::with_capacity(30 + filename.len());
            local_header.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]); // Signature
            local_header.extend_from_slice(&[0x14, 0x00]); // Version
            local_header.extend_from_slice(&[0x00, 0x00]); // Flags
            local_header.extend_from_slice(&[0x00, 0x00]); // Compression
            local_header.extend_from_slice(&[0x00, 0x00]); // Time
            local_header.extend_from_slice(&[0x00, 0x00]); // Date
            local_header.extend_from_slice(&crc.to_le_bytes()); // CRC-32
            local_header.extend_from_slice(&file_len.to_le_bytes()); // Compressed size
            local_header.extend_from_slice(&file_len.to_le_bytes()); // Uncompressed size
            local_header.extend_from_slice(&(filename.len() as u16).to_le_bytes()); // Name length
            local_header.extend_from_slice(&[0x00, 0x00]); // Extra length
            local_header.extend_from_slice(filename); // Filename
            
            // Send header immediately - starts the download!
            if tx.send(Ok(local_header)).await.is_err() {
                return; // Client disconnected
            }
            
            // For the first file, send all data at once to ensure browser recognizes the download
            // For subsequent files, stream in chunks
            if is_first {
                // Send entire first file at once to trigger browser download dialog
                if tx.send(Ok(file_data)).await.is_err() {
                    return; // Client disconnected
                }
            } else {
                // Stream subsequent files in chunks
                const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks
                for chunk in file_data.chunks(CHUNK_SIZE) {
                    if tx.send(Ok(chunk.to_vec())).await.is_err() {
                        return; // Client disconnected
                    }
                }
            }
            
            // Build central directory entry
            let mut cd_entry = Vec::with_capacity(46 + asset.original_filename.len());
            cd_entry.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]); // Signature
            cd_entry.extend_from_slice(&[0x14, 0x00]); // Version made by
            cd_entry.extend_from_slice(&[0x14, 0x00]); // Version needed
            cd_entry.extend_from_slice(&[0x00, 0x00]); // Flags
            cd_entry.extend_from_slice(&[0x00, 0x00]); // Compression
            cd_entry.extend_from_slice(&[0x00, 0x00]); // Time
            cd_entry.extend_from_slice(&[0x00, 0x00]); // Date
            cd_entry.extend_from_slice(&crc.to_le_bytes()); // CRC-32
            cd_entry.extend_from_slice(&file_len.to_le_bytes()); // Compressed
            cd_entry.extend_from_slice(&file_len.to_le_bytes()); // Uncompressed
            cd_entry.extend_from_slice(&(asset.original_filename.len() as u16).to_le_bytes());
            cd_entry.extend_from_slice(&[0x00, 0x00]); // Extra
            cd_entry.extend_from_slice(&[0x00, 0x00]); // Comment
            cd_entry.extend_from_slice(&[0x00, 0x00]); // Disk
            cd_entry.extend_from_slice(&[0x00, 0x00]); // Internal attrs
            cd_entry.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // External attrs
            cd_entry.extend_from_slice(&current_offset.to_le_bytes()); // Offset
            cd_entry.extend_from_slice(asset.original_filename.as_bytes());
            
            central_directory.extend_from_slice(&cd_entry);
            current_offset += 30 + asset.original_filename.len() as u32 + file_len;
        }
        
        // Send central directory
        let cd_len = central_directory.len() as u32;
        if !central_directory.is_empty() {
            let _ = tx.send(Ok(central_directory)).await;
        }
        
        // Send end of central directory record
        let mut end_record = Vec::with_capacity(22);
        end_record.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]); // Signature
        end_record.extend_from_slice(&[0x00, 0x00]); // This disk
        end_record.extend_from_slice(&[0x00, 0x00]); // Central dir disk
        end_record.extend_from_slice(&(assets_for_task.len() as u16).to_le_bytes());
        end_record.extend_from_slice(&(assets_for_task.len() as u16).to_le_bytes());
        end_record.extend_from_slice(&cd_len.to_le_bytes());
        end_record.extend_from_slice(&current_offset.to_le_bytes());
        end_record.extend_from_slice(&[0x00, 0x00]); // Comment length
        
        let _ = tx.send(Ok(end_record)).await;
    });
    
    // Create streaming body that sends data immediately
    let stream = async_stream::stream! {
        while let Some(chunk) = rx.recv().await {
            yield chunk;
        }
    };
    
    let body = Body::from_stream(stream);
    
    // Set response headers for immediate download
    let filename = format!("experiment_{experiment_id}.zip");
    
    // Return response with proper headers for immediate browser download
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "application/zip")
        .header(axum::http::header::CONTENT_DISPOSITION, 
                format!("attachment; filename=\"{filename}\""))
        .header("Content-Length", total_zip_size.to_string())
        .header("X-Accel-Buffering", "no") // Disable nginx buffering
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .header("Expires", "0")
        .body(body)
        .unwrap())
}

/// Process asset data for an experiment
pub async fn process_asset_data(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use sea_orm::Set;

    let asset_id = payload.get("assetId")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing or invalid assetId".to_string()))?;

    // Find the asset
    let asset = s3_assets::Entity::find_by_id(asset_id)
        .one(&app_state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Asset not found".to_string()))?;

    // Update asset status to processing
    let update_asset = s3_assets::ActiveModel {
        id: Set(asset_id),
        processing_status: Set(Some("processing".to_string())),
        processing_message: Set(Some("Processing started...".to_string())),
        ..Default::default()
    };
    let _ = s3_assets::Entity::update(update_asset)
        .exec(&app_state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update asset: {}", e)))?;

    // Download the file from S3 to get bytes for processing (uses mock for tests)
    let file_bytes = crate::external::s3::get_object_from_s3(&asset.s3_key, &app_state.config)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to download from S3: {}", e)))?;

    // Validate file can be processed - only allow Excel files with appropriate names
    let filename = asset.original_filename.to_lowercase();
    let file_extension = std::path::Path::new(&filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    // Check file extension first
    if file_extension != "xlsx" && file_extension != "xls" {
        let error_message = format!("File '{}' is not processable - only Excel files (.xlsx, .xls) with experiment data can be processed", asset.original_filename);
        
        // Update asset with error status
        let update_asset = s3_assets::ActiveModel {
            id: Set(asset_id),
            processing_status: Set(Some("error".to_string())),
            processing_message: Set(Some(error_message.clone())),
            ..Default::default()
        };
        let _ = s3_assets::Entity::update(update_asset)
            .exec(&app_state.db)
            .await;

        return Err((StatusCode::BAD_REQUEST, error_message));
    }
    
    // Check filename content for experiment data files
    if !filename.contains("merged") && !filename.contains("experiment") && !filename.contains("inp freezing") {
        let error_message = format!("File '{}' is not processable - only experiment data files (merged.xlsx, etc.) can be processed", asset.original_filename);
        
        // Update asset with error status
        let update_asset = s3_assets::ActiveModel {
            id: Set(asset_id),
            processing_status: Set(Some("error".to_string())),
            processing_message: Set(Some(error_message.clone())),
            ..Default::default()
        };
        let _ = s3_assets::Entity::update(update_asset)
            .exec(&app_state.db)
            .await;

        return Err((StatusCode::BAD_REQUEST, error_message));
    }

    // Clear any existing processed data and reset other assets' processing status
    // This ensures only one file can have processed data at a time
    use crate::routes::experiments::temperatures::models as temp_models;
    use crate::routes::experiments::phase_transitions::models as phase_models;
    
    // Delete existing temperature readings and phase transitions for this experiment
    let _ = temp_models::Entity::delete_many()
        .filter(temp_models::Column::ExperimentId.eq(experiment_id))
        .exec(&app_state.db)
        .await;
    
    let _ = phase_models::Entity::delete_many()
        .filter(phase_models::Column::ExperimentId.eq(experiment_id))
        .exec(&app_state.db)
        .await;

    // Reset processing status for all other assets in this experiment
    let _ = s3_assets::Entity::update_many()
        .filter(s3_assets::Column::ExperimentId.eq(Some(experiment_id)))
        .filter(s3_assets::Column::Id.ne(asset_id))
        .col_expr(s3_assets::Column::ProcessingStatus, sea_orm::sea_query::Expr::value(sea_orm::Value::String(None)))
        .col_expr(s3_assets::Column::ProcessingMessage, sea_orm::sea_query::Expr::value(sea_orm::Value::String(None)))
        .exec(&app_state.db)
        .await;

    // Process the Excel file
    match app_state.data_processing_service
        .process_excel_file(experiment_id, file_bytes)
        .await {
        Ok(result) => {
            // Check if processing actually succeeded by looking at the result status
            if matches!(result.status, crate::services::models::ProcessingStatus::Completed) && result.temperature_readings_created > 0 {
                let success_message = format!(
                    "Processed {} temperature readings in {}ms", 
                    result.temperature_readings_created,
                    result.processing_time_ms
                );

                // Update asset with success status
                let update_asset = s3_assets::ActiveModel {
                    id: Set(asset_id),
                    processing_status: Set(Some("completed".to_string())),
                    processing_message: Set(Some(success_message.clone())),
                    ..Default::default()
                };
                let _ = s3_assets::Entity::update(update_asset)
                    .exec(&app_state.db)
                    .await;

                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": success_message,
                    "result": result
                })))
            } else {
                // Processing technically succeeded but with errors or no data
                let error_message = result.error.unwrap_or_else(|| {
                    if result.errors.is_empty() {
                        "Processing completed but no temperature readings were created".to_string()
                    } else {
                        result.errors.join("; ")
                    }
                });

                // Update asset with error status
                let update_asset = s3_assets::ActiveModel {
                    id: Set(asset_id),
                    processing_status: Set(Some("error".to_string())),
                    processing_message: Set(Some(error_message.clone())),
                    ..Default::default()
                };
                let _ = s3_assets::Entity::update(update_asset)
                    .exec(&app_state.db)
                    .await;

                Err((StatusCode::UNPROCESSABLE_ENTITY, error_message))
            }
        }
        Err(e) => {
            let error_message = format!("Processing failed: {}", e);

            // Update asset with error status
            let update_asset = s3_assets::ActiveModel {
                id: Set(asset_id),
                processing_status: Set(Some("error".to_string())),
                processing_message: Set(Some(error_message.clone())),
                ..Default::default()
            };
            let _ = s3_assets::Entity::update(update_asset)
                .exec(&app_state.db)
                .await;

            Err((StatusCode::INTERNAL_SERVER_ERROR, error_message))
        }
    }
}

/// Clear all processed results for an experiment
pub async fn clear_experiment_results(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use sea_orm::Set;

    let asset_id = payload.get("assetId")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing or invalid assetId".to_string()))?;

    // Clear processed data by deleting related records directly
    // Delete temperature readings
    use crate::routes::experiments::temperatures::models as temp_models;
    let _ = temp_models::Entity::delete_many()
        .filter(temp_models::Column::ExperimentId.eq(experiment_id))
        .exec(&app_state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to clear temperature readings: {}", e)))?;

    // Delete phase transitions
    use crate::routes::experiments::phase_transitions::models as phase_models;
    let _ = phase_models::Entity::delete_many()
        .filter(phase_models::Column::ExperimentId.eq(experiment_id))
        .exec(&app_state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to clear phase transitions: {}", e)))?;

    // Update asset to remove processing status
    let update_asset = s3_assets::ActiveModel {
        id: Set(asset_id),
        processing_status: Set(None),
        processing_message: Set(None),
        ..Default::default()
    };
    let _ = s3_assets::Entity::update(update_asset)
        .exec(&app_state.db)
        .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Experiment results cleared successfully"
    })))
}
