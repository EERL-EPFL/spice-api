use crate::common::auth::Role;
use crate::common::state::AppState;
use crate::external::s3::get_client;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::{CONTENT_TYPE, CONTENT_DISPOSITION}},
    response::{IntoResponse, Response},
    routing::get,
};
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::CRUDResource;
use sea_orm::EntityTrait;
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;
// crud_handlers!(Asset, AssetUpdate, AssetCreate);
pub use super::models::{Asset, router as crudrouter, Entity as AssetEntity};

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

    // Download file from S3
    let s3_client = crate::external::s3::get_client(&state.config).await;
    let object_result = s3_client
        .get_object()
        .bucket(&state.config.s3_bucket_id)
        .key(&asset.s3_key)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let file_bytes = object_result
        .body
        .collect()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_bytes()
        .to_vec();

    // Process Excel file
    match state.data_processing_service
        .process_excel_file(experiment_id, file_bytes)
        .await {
        Ok(result) => {
            let success_message = format!(
                "Reprocessed {} temperature readings in {}ms", 
                result.temperature_readings_created,
                result.processing_time_ms
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

            Ok(axum::response::Json(serde_json::json!({
                "success": true,
                "message": success_message,
                "temperature_readings_created": result.temperature_readings_created,
                "processing_time_ms": result.processing_time_ms
            })))
        }
        Err(e) => {
            let error_message = format!("Reprocessing failed: {}", e);

            // Update asset with error status
            let update_asset = super::models::ActiveModel {
                id: sea_orm::ActiveValue::Set(id),
                processing_status: sea_orm::ActiveValue::Set(Some("error".to_string())),
                processing_message: sea_orm::ActiveValue::Set(Some(error_message.clone())),
                ..Default::default()
            };
            let _ = AssetEntity::update(update_asset)
                .exec(&state.db)
                .await;

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

    // Download from S3
    let s3_client = get_client(&state.config).await;
    let object_result = s3_client
        .get_object()
        .bucket(&state.config.s3_bucket_id)
        .key(&asset.s3_key)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Stream the body
    let body = object_result
        .body
        .collect()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_bytes();

    // Set headers
    let mut headers = HeaderMap::new();
    
    // Set content type based on file extension or stored type
    let content_type = match asset.r#type.as_str() {
        "image" => {
            let ext = asset.original_filename
                .split('.')
                .last()
                .unwrap_or("")
                .to_lowercase();
            match ext.as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "svg" => "image/svg+xml",
                _ => "image/png",
            }
        },
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

    Ok((headers, body).into_response())
}

pub fn router(state: &AppState) -> OpenApiRouter
where
    Asset: CRUDResource,
{
    let mut mutating_router = crudrouter(&state.db.clone())
        .nest("/{id}", OpenApiRouter::new()
            .route("/download", get(download_asset))
            .route("/view", get(view_asset))
            .route("/reprocess", axum::routing::post(reprocess_asset))
            .with_state(state.clone())
        );
        
    if let Some(instance) = state.keycloak_auth_instance.clone() {
        mutating_router = mutating_router.layer(
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

    mutating_router
}
