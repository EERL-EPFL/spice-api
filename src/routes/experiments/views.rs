use super::models::{Experiment, ExperimentCreate, ExperimentUpdate};
use crate::common::auth::Role;
use crate::common::state::AppState;
use crate::external::s3::{download_assets, get_client};
use aws_sdk_s3::primitives::ByteStream;
use axum::body::Body;
use axum::routing::post;
use axum::{extract::Multipart, response::Response, routing::get};
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::{CRUDResource, crud_handlers};
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::ConnectionTrait;
use serde::Serialize;
use spice_entity::s3_assets;
use std::convert::TryInto;
use tokio_util::io::ReaderStream;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

crud_handlers!(Experiment, ExperimentUpdate, ExperimentCreate);

pub fn router(state: &AppState) -> OpenApiRouter
where
    Experiment: CRUDResource,
{
    let mut mutating_router = OpenApiRouter::new()
        .routes(routes!(get_one_handler))
        .routes(routes!(get_all_handler))
        .routes(routes!(create_one_handler))
        .routes(routes!(update_one_handler))
        .routes(routes!(delete_one_handler))
        .routes(routes!(delete_many_handler))
        .with_state(state.db.clone())
        .route("/{experiment_id}/uploads", post(upload_file))
        .route("/{experiment_id}/download", get(download_experiment_assets))
        .route(
            "/{experiment_id}/process-excel",
            post(super::excel_upload::process_excel_upload),
        )
        .with_state(state.clone());

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

#[derive(Serialize, ToSchema)]
pub struct UploadResponse {
    success: bool,
    filename: String,
    size: u64,
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
    if spice_entity::experiments::Entity::find_by_id(experiment_id)
        .one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_none()
    {
        return Err((StatusCode::NOT_FOUND, "Experiment not found".to_string()));
    }

    // Load S3 configuration from environment
    let config = crate::config::Config::from_env();
    let s3_client = get_client(&config).await;

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
            config.app_name, config.deployment, experiment_id, file_name
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
            .bucket(&config.s3_bucket_id)
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

        // Insert a record into the local DB
        let asset = s3_assets::ActiveModel {
            original_filename: Set(file_name.clone()),
            experiment_id: Set(Some(experiment_id)),
            s3_key: Set(s3_key.clone()),
            size_bytes: Set(Some(size.try_into().unwrap())),
            uploaded_by: Set(Some("uploader".to_string())),
            r#type: Set(file_type),
            role: Set(Some("raw_image".to_string())),
            ..Default::default()
        };
        s3_assets::Entity::insert(asset)
            .exec(&state.db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to insert asset record: {e}"),
                )
            })?;

        return Ok(Json(UploadResponse {
            success: true,
            filename: file_name,
            size,
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
