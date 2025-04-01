use super::models::{Experiment, ExperimentCreate, ExperimentUpdate};
use crate::common::auth::Role;
use crate::external::s3::get_client;
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};
use axum::{extract::Multipart, routing::post};
use axum_keycloak_auth::{
    PassthroughMode, instance::KeycloakAuthInstance, layer::KeycloakAuthLayer,
};
use crudcrate::{CRUDResource, crud_handlers};
use sea_orm::DatabaseConnection;
use sea_orm::entity::prelude::*;
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
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
#[axum::debug_handler]
pub async fn upload_file(
    State(db): State<DatabaseConnection>,
    Path(experiment_id): Path<Uuid>,
    mut infile: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    // Check if the experiment exists
    super::db::Entity::find_by_id(experiment_id)
        .one(&db)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Experiment not found".to_string()))?;

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
        let mut file_bytes = Vec::new();
        while let Some(chunk) = field.chunk().await.unwrap() {
            file_bytes.extend_from_slice(&chunk);
        }
        let size = file_bytes.len() as u64;

        // Generate a unique S3 key (you might choose to append an extension based on file_name)
        let s3_key = format!("experiments/{experiment_id}/{file_name}");

        // Upload the file to S3.
        let body = ByteStream::from(file_bytes.clone());
        match s3_client
            .put_object()
            .bucket(&config.s3_bucket_id)
            .key(&s3_key)
            .body(body)
            .send()
            .await
        {
            Ok(_) => {
                println!("Uploaded file: {} with size: {} to S3", file_name, size);
            }
            Err(e) => {
                println!("Error uploading file to S3: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to upload to S3".to_string(),
                ));
            }
        }

        // Insert a record into the local DB.
        // Assumes your SeaORM entity for s3_assets is located in crate::routes::trays::db::s3_assets.
        use crate::routes::s3::db as s3_assets;
        use sea_orm::ActiveModelTrait;
        use sea_orm::ActiveValue::Set;

        let asset = s3_assets::ActiveModel {
            original_filename: Set(file_name.clone()),
            s3_key: Set(s3_key.clone()),
            size_bytes: Set(Some(size as i64)),
            uploaded_by: Set(Some("uploader".to_string())), // Replace with the actual uploader if available
            // For this example, we use fixed values for type and role.
            r#type: Set("image".to_string()),
            role: Set(Some("raw_image".to_string())),
            ..Default::default()
        };
        match s3_assets::Entity::insert(asset).exec(&db).await {
            Ok(res) => {
                println!("Inserted S3 asset record with id: {:?}", res.last_insert_id);
            }
            Err(e) => {
                println!("Error inserting S3 asset record: {e}");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to insert asset record".to_string(),
                ));
            }
        }

        return Ok(Json(UploadResponse {
            success: true,
            filename: file_name,
            size,
        }));
    }

    Err((StatusCode::BAD_REQUEST, "No file uploaded".to_string()))
}

crud_handlers!(Experiment, ExperimentUpdate, ExperimentCreate);

pub fn router(
    db: &DatabaseConnection,
    keycloak_auth_instance: Option<Arc<KeycloakAuthInstance>>,
) -> OpenApiRouter
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
        .route("/uploads", post(upload_file))
        .with_state(db.clone());

    if let Some(instance) = keycloak_auth_instance {
        mutating_router = mutating_router.layer(
            KeycloakAuthLayer::<Role>::builder()
                .instance(instance)
                .passthrough_mode(PassthroughMode::Block)
                .persist_raw_claims(false)
                .expected_audiences(vec![String::from("account")])
                .required_roles(vec![Role::Administrator])
                .build(),
        );
    } else {
        println!(
            "Warning: Mutating routes of {} router are not protected",
            Experiment::RESOURCE_NAME_PLURAL
        );
    }

    mutating_router
}
