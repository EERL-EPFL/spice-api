use super::models::{Experiment, ExperimentCreate, ExperimentUpdate};
use crate::common::auth::Role;
use axum::extract::Multipart;
use axum::routing::post;
use axum_keycloak_auth::{
    PassthroughMode, instance::KeycloakAuthInstance, layer::KeycloakAuthLayer,
};
use crudcrate::{CRUDResource, crud_handlers};
use sea_orm::DatabaseConnection;
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
    path = "/uploads",
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
    mut infile: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    while let Some(mut field) = infile.next_field().await.unwrap() {
        let field_name = field.name().unwrap_or("none").to_string();
        let file_name = field.file_name().unwrap_or("unknown").to_string();
        // println!("Field name: {}", field_name);
        // println!("File name: {}", file_name);
        // println!("Field headers: {:?}", field.headers());

        let mut size: u64 = 0;
        while let Some(chunk) = field.chunk().await.unwrap() {
            size += chunk.len() as u64;
        }

        // If you're expecting a particular field name, you can check it here:
        if field_name == "file" {
            println!("Uploaded file: {file_name} with size: {size}. Field name: {field_name}");

            return Ok(Json(UploadResponse {
                success: true,
                filename: file_name,
                size,
            }));
        }
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
