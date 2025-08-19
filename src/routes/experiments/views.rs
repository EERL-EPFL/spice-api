pub use super::models::{Experiment, router as crudrouter};
use crate::common::auth::Role;
use crate::common::models::ProcessingStatus;
use crate::common::state::AppState;
use crate::external::s3::get_client;
use crate::routes::assets::models as s3_assets;
use crate::routes::experiments::phase_transitions::models as phase_models;
use crate::routes::experiments::temperatures::models as temp_models;
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{
    Router,
    extract::Multipart,
    http::{HeaderMap, status::StatusCode},
    response::{Json, Response},
    routing::get,
};
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

// Helper struct for file upload processing
struct FileUploadData {
    file_name: String,
    file_bytes: Vec<u8>,
    file_type: String,
    extension: String,
    size: u64,
    s3_key: String,
}

// Helper struct for asset processing results
struct AssetProcessingResult {
    auto_processed: bool,
    processing_message: Option<String>,
}

/// Process multipart field into file upload data
async fn process_multipart_field(
    field: &mut axum::extract::multipart::Field<'_>,
    experiment_id: Uuid,
    state: &AppState,
) -> Result<FileUploadData, (StatusCode, String)> {
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

    let s3_key = format!(
        "{}/{}/experiments/{}/{}",
        state.config.app_name, state.config.deployment, experiment_id, file_name
    );

    Ok(FileUploadData {
        file_name,
        file_bytes,
        file_type,
        extension,
        size,
        s3_key,
    })
}

/// Handle existing asset overwrite logic
async fn handle_existing_asset(
    file_name: &str,
    experiment_id: Uuid,
    allow_overwrite: bool,
    state: &AppState,
) -> Result<(), (StatusCode, String)> {
    let existing_asset = s3_assets::Entity::find()
        .filter(s3_assets::Column::ExperimentId.eq(Some(experiment_id)))
        .filter(s3_assets::Column::OriginalFilename.eq(file_name))
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
        if let Err(e) = crate::external::s3::delete_from_s3(&existing.s3_key).await {
            println!(
                "Warning: Failed to delete existing file from S3: {} - {}",
                existing.s3_key, e
            );
        }

        s3_assets::Entity::delete_by_id(existing.id)
            .exec(&state.db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to delete existing asset: {e}"),
                )
            })?;
    }
    Ok(())
}

/// Process Excel file if it should be auto-processed
async fn process_excel_if_needed(
    upload_data: &FileUploadData,
    asset_id: Uuid,
    experiment_id: Uuid,
    state: &AppState,
) -> AssetProcessingResult {
    let asset_role = determine_asset_role(
        &upload_data.file_name,
        &upload_data.file_type,
        &upload_data.extension,
    );

    let should_auto_process = upload_data.file_type == "tabular"
        && (upload_data.extension == "xlsx" || upload_data.extension == "xls")
        && asset_role == "analysis_data";

    if !should_auto_process {
        return AssetProcessingResult {
            auto_processed: false,
            processing_message: None,
        };
    }

    println!("🔄 Auto-processing Excel file: {}", upload_data.file_name);

    let processing_service =
        crate::services::data_processing_service::DataProcessingService::new(state.db.clone());

    match processing_service
        .process_excel_file(experiment_id, upload_data.file_bytes.clone())
        .await
    {
        Ok(result) => {
            let processing_status = match result.status {
                crate::common::models::ProcessingStatus::Completed => Some("completed".to_string()),
                crate::common::models::ProcessingStatus::Failed => Some("error".to_string()),
                _ => Some("processing".to_string()),
            };

            let message = if result.status == crate::common::models::ProcessingStatus::Completed {
                Some(format!(
                    "✅ Processed {} temperature readings in {}ms",
                    result.temperature_readings_created, result.processing_time_ms
                ))
            } else if let Some(error) = result.error {
                Some(format!("❌ Processing failed: {error}"))
            } else {
                Some("Processing completed".to_string())
            };

            let _ = crate::routes::assets::models::Entity::update_many()
                .col_expr(
                    crate::routes::assets::models::Column::ProcessingStatus,
                    sea_orm::sea_query::Expr::value(processing_status),
                )
                .col_expr(
                    crate::routes::assets::models::Column::ProcessingMessage,
                    sea_orm::sea_query::Expr::value(message.clone()),
                )
                .filter(crate::routes::assets::models::Column::Id.eq(asset_id))
                .exec(&state.db)
                .await;

            AssetProcessingResult {
                auto_processed: true,
                processing_message: message,
            }
        }
        Err(e) => {
            let error_msg = format!("❌ Auto-processing failed: {e}");
            println!("{error_msg}");

            let _ = crate::routes::assets::models::Entity::update_many()
                .col_expr(
                    crate::routes::assets::models::Column::ProcessingStatus,
                    sea_orm::sea_query::Expr::value(Some("error".to_string())),
                )
                .col_expr(
                    crate::routes::assets::models::Column::ProcessingMessage,
                    sea_orm::sea_query::Expr::value(Some(error_msg.clone())),
                )
                .filter(crate::routes::assets::models::Column::Id.eq(asset_id))
                .exec(&state.db)
                .await;

            AssetProcessingResult {
                auto_processed: false,
                processing_message: Some(error_msg),
            }
        }
    }
}

/// Determine asset role based on filename patterns and file type
fn determine_asset_role(filename: &str, file_type: &str, _extension: &str) -> String {
    let filename_lower = filename.to_lowercase();

    match file_type {
        "image" => {
            // Camera images from INP system follow pattern: INP_XXXXX_YYYY-MM-DD_HH-MM-SS
            if filename_lower.starts_with("inp_")
                && (filename_lower.contains("2024")
                    || filename_lower.contains("2025")
                    || filename_lower.contains("2026"))
            {
                "camera_image".to_string()
            } else if filename_lower.contains("analysis")
                || filename_lower.contains("frozen_fraction")
                || filename_lower.contains("regions")
                || filename_lower.contains("trays_config")
            {
                "analysis_data".to_string()
            } else {
                "other_image".to_string()
            }
        }
        "tabular" => {
            if filename_lower.contains("freezing_temperatures")
                || (filename_lower.contains("merged")
                    && (filename_lower.contains("csv") || filename_lower.contains("xlsx")))
                || filename_lower.contains("analysis")
            {
                "analysis_data".to_string()
            } else if filename_lower.contains("inp") && filename_lower.contains("freezing") {
                "temperature_data".to_string()
            } else if filename_lower.contains("config")
                || filename_lower.contains("setup")
                || filename_lower.contains("yaml")
            {
                "configuration".to_string()
            } else {
                "raw_data".to_string()
            }
        }
        "netcdf" => {
            if filename_lower.contains("analysis") || filename_lower.contains("well_temperatures") {
                "analysis_data".to_string()
            } else {
                "scientific_data".to_string()
            }
        }
        _ => {
            if filename_lower.contains("readme") || filename_lower.contains("doc") {
                "documentation".to_string()
            } else if filename_lower.contains("config")
                || filename_lower.contains("yaml")
                || filename_lower.contains("yml")
            {
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
        .route("/{experiment_id}/process-asset", post(process_asset_data))
        .route(
            "/{experiment_id}/clear-results",
            post(clear_experiment_results),
        )
        .route("/{experiment_id}/results", get(get_experiment_results))
        .layer(DefaultBodyLimit::max(30 * 1024 * 1024)) // 30MB limit to match main router
}

/// Separate router for asset upload/download endpoints (uses regular Axum Router, not `OpenApiRouter`)
pub fn asset_router() -> Router<AppState> {
    use axum::extract::DefaultBodyLimit;

    Router::new()
        .route("/{experiment_id}/uploads", post(upload_file))
        .route("/{experiment_id}/download", get(download_experiment_assets))
        .route(
            "/{experiment_id}/download-token",
            post(create_experiment_download_token),
        )
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

        // Process multipart field into structured data
        let upload_data = process_multipart_field(&mut field, experiment_id, &state).await?;

        // Check if overwrite is allowed
        let allow_overwrite = headers
            .get("x-allow-overwrite")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|s| s == "true");

        // Handle existing asset overwrite logic
        handle_existing_asset(
            &upload_data.file_name,
            experiment_id,
            allow_overwrite,
            &state,
        )
        .await?;

        // Upload the file to S3 (uses mock for tests, real S3 for production)
        if let Err(e) = crate::external::s3::put_object_to_s3(
            &upload_data.s3_key,
            upload_data.file_bytes.clone(),
            &state.config,
        )
        .await
        {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to upload to S3: {e}"),
            ));
        }

        // Determine asset role based on filename patterns and type
        let asset_role = determine_asset_role(
            &upload_data.file_name,
            &upload_data.file_type,
            &upload_data.extension,
        );

        // Insert a record into the local DB
        let asset = s3_assets::ActiveModel {
            original_filename: Set(upload_data.file_name.clone()),
            experiment_id: Set(Some(experiment_id)),
            s3_key: Set(upload_data.s3_key.clone()),
            size_bytes: Set(Some(upload_data.size.try_into().unwrap())),
            uploaded_by: Set(Some("uploader".to_string())),
            r#type: Set(upload_data.file_type.clone()),
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

        // Process Excel file if needed using helper function
        let processing_result =
            process_excel_if_needed(&upload_data, asset_id, experiment_id, &state).await;

        return Ok(Json(UploadResponse {
            success: true,
            filename: upload_data.file_name,
            size: upload_data.size,
            auto_processed: processing_result.auto_processed,
            processing_message: processing_result.processing_message,
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
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

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

    crate::routes::assets::services::create_hybrid_streaming_zip_response(assets, &state.config)
        .await
        .map(|mut response| {
            // Update filename for experiment
            let headers = response.headers_mut();
            headers.insert(
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"experiment_{experiment_id}.zip\"")
                    .parse()
                    .unwrap(),
            );
            response
        })
}

/// Process asset data for an experiment
#[allow(clippy::too_many_lines)]
pub async fn process_asset_data(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use sea_orm::Set;

    let asset_id = payload
        .get("assetId")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Missing or invalid assetId".to_string(),
            )
        })?;

    // Find the asset
    let asset = s3_assets::Entity::find_by_id(asset_id)
        .one(&app_state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            )
        })?
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
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update asset: {e}"),
            )
        })?;

    // Download the file from S3 to get bytes for processing (uses mock for tests)
    let file_bytes = crate::external::s3::get_object_from_s3(&asset.s3_key, &app_state.config)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to download from S3: {e}"),
            )
        })?;

    // Validate file can be processed - only allow Excel files with appropriate names
    let filename = asset.original_filename.to_lowercase();
    let file_extension = std::path::Path::new(&filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    // Check file extension first
    if file_extension != "xlsx" && file_extension != "xls" {
        let error_message = format!(
            "File '{}' is not processable - only Excel files (.xlsx, .xls) with experiment data can be processed",
            asset.original_filename
        );

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
    if !filename.contains("merged")
        && !filename.contains("experiment")
        && !filename.contains("inp freezing")
    {
        let error_message = format!(
            "File '{}' is not processable - only experiment data files (merged.xlsx, etc.) can be processed",
            asset.original_filename
        );

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
        .col_expr(
            s3_assets::Column::ProcessingStatus,
            sea_orm::sea_query::Expr::value(sea_orm::Value::String(None)),
        )
        .col_expr(
            s3_assets::Column::ProcessingMessage,
            sea_orm::sea_query::Expr::value(sea_orm::Value::String(None)),
        )
        .exec(&app_state.db)
        .await;

    // Process the Excel file
    match app_state
        .data_processing_service
        .process_excel_file(experiment_id, file_bytes)
        .await
    {
        Ok(result) => {
            // Check if processing actually succeeded by looking at the result status
            if matches!(result.status, ProcessingStatus::Completed)
                && result.temperature_readings_created > 0
            {
                let success_message = format!(
                    "Processed {} temperature readings in {}ms",
                    result.temperature_readings_created, result.processing_time_ms
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
            let error_message = format!("Processing failed: {e}");

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

    let asset_id = payload
        .get("assetId")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Missing or invalid assetId".to_string(),
            )
        })?;

    // Clear processed data by deleting related records directly
    // Delete temperature readings
    let _ = temp_models::Entity::delete_many()
        .filter(temp_models::Column::ExperimentId.eq(experiment_id))
        .exec(&app_state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to clear temperature readings: {e}"),
            )
        })?;

    // Delete phase transitions

    let _ = phase_models::Entity::delete_many()
        .filter(phase_models::Column::ExperimentId.eq(experiment_id))
        .exec(&app_state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to clear phase transitions: {e}"),
            )
        })?;

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

/// Get experiment results in tray-centric format
#[utoipa::path(
    get,
    path = "/{experiment_id}/results",
    responses(
        (status = 200, description = "Experiment results in tray-centric format", body = super::models::ExperimentResultsResponse),
        (status = 404, description = "Experiment not found", body = String),
        (status = 500, description = "Internal Server Error", body = String)
    ),
    operation_id = "get_experiment_results",
    summary = "Get experiment results in tray-centric format",
    description = "Returns experiment results organized by trays with direct well information including sample and treatment data"
)]
pub async fn get_experiment_results(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
) -> Result<Json<super::models::ExperimentResultsResponse>, (StatusCode, String)> {
    // Check if experiment exists
    let _experiment = super::models::Entity::find_by_id(experiment_id)
        .one(&app_state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Experiment not found".to_string()))?;

    // Build tray-centric results
    let results = super::services::build_tray_centric_results(experiment_id, &app_state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build results: {e}"),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "No results available for this experiment".to_string(),
            )
        })?;

    Ok(Json(results))
}
