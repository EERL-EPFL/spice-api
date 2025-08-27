use crate::common::auth::Role;
use crate::common::state::AppState;

use crate::assets::models as s3_assets;
use axum::{
    extract::{Path, State},
    http::{
        HeaderMap, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::CRUDResource;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;
// crud_handlers!(Asset, AssetUpdate, AssetCreate);
pub use super::models::{Asset, Entity as AssetEntity, router as crudrouter};

/// Download an asset as an attachment
#[utoipa::path(
    get,
    path = "/{id}/download",
    params(
        ("id" = Uuid, Path, description = "Asset ID to download")
    ),
    responses(
        (status = 200, description = "Asset downloaded successfully"),
        (status = 404, description = "Asset not found"),
        (status = 500, description = "Failed to retrieve asset from S3")
    ),
    tag = "assets"
)]
async fn download_asset(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    serve_asset_internal(id, &state, true).await
}

/// View an asset inline (for images, etc.)
#[utoipa::path(
    get,
    path = "/{id}/view",
    params(
        ("id" = Uuid, Path, description = "Asset ID to view")
    ),
    responses(
        (status = 200, description = "Asset displayed inline"),
        (status = 404, description = "Asset not found"),
        (status = 500, description = "Failed to retrieve asset from S3")
    ),
    tag = "assets"
)]
async fn view_asset(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    serve_asset_internal(id, &state, false).await
}


/// Reprocess an Excel asset (for merged.xlsx files)
#[utoipa::path(
    post,
    path = "/{id}/reprocess",
    params(
        ("id" = Uuid, Path, description = "Asset ID to reprocess")
    ),
    responses(
        (status = 200, description = "Asset reprocessing started"),
        (status = 404, description = "Asset not found"),
        (status = 400, description = "Asset is not a processable Excel file"),
        (status = 500, description = "Failed to start reprocessing")
    ),
    tag = "assets"
)]
async fn reprocess_asset(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<axum::response::Json<serde_json::Value>, StatusCode> {
    // Find the asset in the database
    let asset = AssetEntity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if this is a processable Excel file
    let is_merged_xlsx = (asset.original_filename.eq_ignore_ascii_case("merged.xlsx")
        || asset.original_filename.to_lowercase().contains("merged"))
        && asset.r#type == "tabular"
        && asset.original_filename.to_lowercase().ends_with(".xlsx");

    if !is_merged_xlsx {
        return Err(StatusCode::BAD_REQUEST);
    }

    let experiment_id = asset.experiment_id.ok_or(StatusCode::BAD_REQUEST)?;

    // Update status to processing
    let update_asset = super::models::ActiveModel {
        id: sea_orm::ActiveValue::Set(id),
        processing_status: sea_orm::ActiveValue::Set(Some("processing".to_string())),
        processing_message: sea_orm::ActiveValue::Set(Some("Reprocessing started...".to_string())),
        ..Default::default()
    };
    AssetEntity::update(update_asset)
        .exec(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Download file from S3 (uses mock for tests, real S3 for production)
    let file_bytes = crate::external::s3::get_object_from_s3(&asset.s3_key, &state.config)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Process Excel file
    match state
        .data_processing_service
        .process_excel_file(experiment_id, file_bytes)
        .await
    {
        Ok(result) => {
            let success_message = format!(
                "Reprocessed {} temperature readings in {}ms",
                result.temperature_readings_created, result.processing_time_ms
            );

            // Update asset with success status
            let update_asset = super::models::ActiveModel {
                id: sea_orm::ActiveValue::Set(id),
                processing_status: sea_orm::ActiveValue::Set(Some("completed".to_string())),
                processing_message: sea_orm::ActiveValue::Set(Some(success_message.clone())),
                ..Default::default()
            };
            AssetEntity::update(update_asset)
                .exec(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(axum::response::Json(serde_json::to_value(&result).unwrap()))
        }
        Err(e) => {
            let error_message = format!("Reprocessing failed: {e}");

            // Update asset with error status
            let update_asset = super::models::ActiveModel {
                id: sea_orm::ActiveValue::Set(id),
                processing_status: sea_orm::ActiveValue::Set(Some("error".to_string())),
                processing_message: sea_orm::ActiveValue::Set(Some(error_message.clone())),
                ..Default::default()
            };
            let _ = AssetEntity::update(update_asset).exec(&state.db).await;

            Ok(axum::response::Json(serde_json::json!({
                "success": false,
                "message": error_message
            })))
        }
    }
}

async fn serve_asset_internal(
    id: Uuid,
    state: &AppState,
    as_attachment: bool,
) -> Result<Response, StatusCode> {
    // Find the asset in the database
    let asset = AssetEntity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Download from S3 (uses mock for tests, real S3 for production)
    let body_bytes = crate::external::s3::get_object_from_s3(&asset.s3_key, &state.config)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Set headers
    let mut headers = HeaderMap::new();

    // Set content type based on file extension or stored type
    let content_type = match asset.r#type.as_str() {
        "image" => {
            let ext = asset
                .original_filename
                .split('.')
                .next_back()
                .unwrap_or("")
                .to_lowercase();
            match ext.as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "svg" => "image/svg+xml",
                _ => {
                    // Default to application/octet-stream for unknown image types
                    "application/octet-stream"
                }
            }
        }
        "tabular" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "netcdf" => "application/x-netcdf",
        _ => "application/octet-stream",
    };

    headers.insert(CONTENT_TYPE, content_type.parse().unwrap());

    if as_attachment {
        let disposition = format!("attachment; filename=\"{}\"", asset.original_filename);
        headers.insert(CONTENT_DISPOSITION, disposition.parse().unwrap());
    } else {
        let disposition = format!("inline; filename=\"{}\"", asset.original_filename);
        headers.insert(CONTENT_DISPOSITION, disposition.parse().unwrap());
    }

    Ok((headers, body_bytes).into_response())
}

/// Create a download token for bulk asset download
#[utoipa::path(
    post,
    path = "/bulk-download-token",
    request_body(content_type = "application/json", description = "Asset IDs to download"),
    responses(
        (status = 200, description = "Download token created"),
        (status = 400, description = "Invalid request")
    ),
    tag = "assets"
)]
async fn create_bulk_download_token(
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, String)> {
    let asset_ids = payload
        .get("asset_ids")
        .and_then(|v| v.as_array())
        .ok_or((StatusCode::BAD_REQUEST, "Missing asset_ids".to_string()))?;

    let asset_uuids: Result<Vec<Uuid>, _> = asset_ids
        .iter()
        .map(|v| v.as_str().unwrap_or("").parse::<Uuid>())
        .collect();

    let asset_uuids =
        asset_uuids.map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset IDs".to_string()))?;

    if asset_uuids.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No asset IDs provided".to_string()));
    }

    let token = state.create_download_token(asset_uuids).await;

    Ok(axum::Json(serde_json::json!({
        "token": token,
        "download_url": format!("/api/assets/download/{}", token)
    })))
}

/// Download assets using a token (GET endpoint for direct browser download)
#[utoipa::path(
    get,
    path = "/download/{token}",
    params(
        ("token" = String, Path, description = "Download token")
    ),
    responses(
        (status = 200, description = "ZIP file with assets"),
        (status = 404, description = "Invalid or expired token"),
        (status = 500, description = "Failed to create ZIP file")
    ),
    tag = "assets"
)]
async fn download_with_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    // Consume the token (it's single-use)
    let download_token = state.consume_download_token(&token).await.ok_or((
        StatusCode::NOT_FOUND,
        "Invalid or expired token".to_string(),
    ))?;

    // Handle experiment download
    if let Some(experiment_id) = download_token.experiment_id {
        // Fetch experiment assets
        let assets = s3_assets::Entity::find()
            .filter(s3_assets::Column::ExperimentId.eq(Some(experiment_id)))
            .all(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            })?;

        if assets.is_empty() {
            return Err((
                StatusCode::NOT_FOUND,
                "No assets found for experiment".to_string(),
            ));
        }

        // Use hybrid streaming: concurrent downloads + immediate streaming
        let mut response =
            super::services::create_hybrid_streaming_zip_response(assets, &state.config).await?;

        // Update filename for experiment
        let headers = response.headers_mut();
        headers.insert(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"experiment_{experiment_id}.zip\"")
                .parse()
                .unwrap(),
        );

        return Ok(response);
    }

    // Handle regular asset download
    let asset_uuids = download_token.asset_ids;
    if asset_uuids.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No assets in token".to_string()));
    }

    // Fetch assets from database
    let assets: Vec<super::models::Model> = AssetEntity::find()
        .filter(super::models::Column::Id.is_in(asset_uuids))
        .all(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    if assets.is_empty() {
        return Err((StatusCode::NOT_FOUND, "No assets found".to_string()));
    }

    // Use hybrid streaming: concurrent downloads + immediate streaming
    super::services::create_hybrid_streaming_zip_response(assets, &state.config).await
}

pub fn router(state: &AppState) -> OpenApiRouter
where
    Asset: CRUDResource,
{
    // Public routes (no authentication required) - token-based downloads
    let public_router = OpenApiRouter::new().route(
        "/download/{token}",
        get(download_with_token).with_state(state.clone()),
    );

    // Authenticated routes - token creation and other operations
    let mut authenticated_router = crudrouter(&state.db.clone())
        .nest(
            "/{id}",
            OpenApiRouter::new()
                .route("/download", get(download_asset))
                .route("/view", get(view_asset))
                .route("/reprocess", axum::routing::post(reprocess_asset))
                .with_state(state.clone()),
        )
        .route(
            "/bulk-download-token",
            post(create_bulk_download_token).with_state(state.clone()),
        );

    // Apply authentication to the authenticated routes only
    if let Some(instance) = state.keycloak_auth_instance.clone() {
        authenticated_router = authenticated_router.layer(
            KeycloakAuthLayer::<Role>::builder()
                .instance(instance)
                .passthrough_mode(PassthroughMode::Block)
                .persist_raw_claims(false)
                .expected_audiences(vec![String::from("account")])
                .required_roles(vec![Role::Administrator])
                .build(),
        );
    } else if !state.config.tests_running {
        println!(
            "Warning: Mutating routes of {} router are not protected",
            Asset::RESOURCE_NAME_PLURAL
        );
    }

    // Merge public and authenticated routers
    public_router.merge(authenticated_router)
}
