pub use super::models::{Experiment, router as crudrouter};
use crate::common::auth::Role;
use crate::common::state::AppState;
use crate::external::s3::{download_assets, get_client};
use crate::routes::assets::models as s3_assets;
use aws_sdk_s3::primitives::ByteStream;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{extract::Multipart, http::{status::StatusCode, HeaderMap}, response::{Json, Response}, routing::get, Router};
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::CRUDResource;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use serde::Serialize;
use std::convert::TryInto;
use tokio_util::io::ReaderStream;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use http_body_util;

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

    // Load S3 configuration from app state
    let s3_client = get_client(&state.config).await;

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

        // Check if file already exists in database
        let existing_asset = s3_assets::Entity::find()
            .filter(s3_assets::Column::ExperimentId.eq(Some(experiment_id)))
            .filter(s3_assets::Column::OriginalFilename.eq(&file_name))
            .one(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if existing_asset.is_some() {
            return Err((
                StatusCode::CONFLICT,
                format!("File '{file_name}' already exists in this experiment"),
            ));
        }

        // Upload the file to S3
        let body = ByteStream::from(file_bytes.clone());
        if s3_client
            .put_object()
            .bucket(&state.config.s3_bucket_id)
            .key(&s3_key)
            .body(body)
            .send()
            .await
            .is_err()
        {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to upload to S3".to_string(),
            ));
        }

        // Determine if this is a merged.xlsx file that should be processed
        let is_merged_xlsx = (file_name.eq_ignore_ascii_case("merged.xlsx") || file_name.to_lowercase().contains("merged")) 
            && file_type == "tabular" 
            && extension == "xlsx";

        // Insert a record into the local DB
        let asset = s3_assets::ActiveModel {
            original_filename: Set(file_name.clone()),
            experiment_id: Set(Some(experiment_id)),
            s3_key: Set(s3_key.clone()),
            size_bytes: Set(Some(size.try_into().unwrap())),
            uploaded_by: Set(Some("uploader".to_string())),
            r#type: Set(file_type.clone()),
            role: Set(Some(if is_merged_xlsx { "experiment_data".to_string() } else { "raw_data".to_string() })),
            processing_status: Set(if is_merged_xlsx { Some("processing".to_string()) } else { None }),
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

        // Disabled auto-processing - let users manually trigger processing
        let auto_processed = false;
        let processing_message = None;

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

    // Load configuration and create S3 client.
    let s3_client = get_client(&state.config).await;

    // Call the new function to download assets concurrently.
    let (_temp_dir, asset_paths) = download_assets(assets, &state.config, s3_client).await?;

    // Create a temporary file for the zip archive.
    let zip_temp_file = tempfile::NamedTempFile::new()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let zip_path = zip_temp_file.path().to_owned();
    let zip_path_for_task = zip_path.clone(); // Clone the path for the blocking task
    drop(zip_temp_file);

    // Create the zip archive in a blocking task.
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let zip_file = std::fs::File::create(&zip_path_for_task).map_err(|e| e.to_string())?;
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let options = zip::write::FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o644);
        for (file_name, file_path) in asset_paths {
            zip_writer
                .start_file(file_name, options)
                .map_err(|e| e.to_string())?;
            let mut f = std::fs::File::open(&file_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut zip_writer).map_err(|e| e.to_string())?;
        }
        zip_writer.finish().map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Open the zip file asynchronously for streaming.
    let file = tokio::fs::File::open(&zip_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let stream = ReaderStream::new(file);
    let body_stream = http_body_util::StreamBody::new(stream);
    let hyper_body = Body::from_stream(body_stream);

    // Build response headers.
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "application/zip".parse().unwrap(),
    );
    let filename = format!("experiment_{experiment_id}.zip",);
    let content_disposition = format!("attachment; filename=\"{filename}\"",);
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        content_disposition.parse().unwrap(),
    );

    let mut response_builder = Response::builder().status(StatusCode::OK);
    for (key, value) in &headers {
        response_builder = response_builder.header(key, value);
    }
    Ok(response_builder.body(hyper_body).unwrap())
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

    // Download the file from S3 to get bytes for processing
    let s3_client = get_client(&app_state.config).await;
    let get_object_output = s3_client
        .get_object()
        .bucket(&app_state.config.s3_bucket_id)
        .key(&asset.s3_key)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to download from S3: {}", e)))?;

    let file_bytes = get_object_output
        .body
        .collect()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read S3 data: {}", e)))?
        .into_bytes()
        .to_vec();

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
