use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::Json,
};
use sea_orm::EntityTrait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::common::state::AppState;
use crate::services::processing::excel_processor::ExcelProcessingResult;

/// Upload and process an Excel file containing merged experiment data
#[utoipa::path(
    post,
    path = "/experiments/{experiment_id}/process-excel",
    request_body(content = String, description = "Excel file as multipart/form-data", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Excel file processed successfully", body = ExcelProcessingResult),
        (status = 400, description = "Invalid Excel file or format"),
        (status = 404, description = "Experiment not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("experiment_id" = Uuid, Path, description = "Experiment ID")
    ),
    tag = "experiments"
)]
pub async fn process_excel_upload(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<ExcelProcessingResult>, (StatusCode, Json<Value>)> {
    let db = &app_state.db;

    // Verify experiment exists
    let experiment = crate::experiments::models::Entity::find_by_id(experiment_id)
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Database error: {}", e)
                })),
            )
        })?;

    if experiment.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Experiment not found"
            })),
        ));
    }

    // Extract Excel file from multipart upload
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Multipart error: {}", e)
            })),
        )
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "excel_file" || field_name == "file" {
            file_name = field.file_name().map(std::string::ToString::to_string);
            file_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": format!("Failed to read file data: {}", e)
                            })),
                        )
                    })?
                    .to_vec(),
            );
            break;
        }
    }

    let file_data = file_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "No Excel file found in request"
            })),
        )
    })?;

    let file_name = file_name.unwrap_or_else(|| "uploaded_file.xlsx".to_string());

    // Validate file format
    if !std::path::Path::new(&file_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xlsx"))
        && !std::path::Path::new(&file_name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xls"))
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "File must be an Excel file (.xlsx or .xls)"
            })),
        ));
    }

    // Process the Excel file using service layer
    let result = app_state
        .data_processing_service
        .process_excel_file(experiment_id, file_data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Excel processing error: {}", e)
                })),
            )
        })?;

    Ok(Json(result))
}
