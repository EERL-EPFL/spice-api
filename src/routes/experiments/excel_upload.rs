use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::Json,
};
use sea_orm::EntityTrait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::common::state::AppState;
use crate::services::data_processing_service::ExcelProcessingResult;

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
    let experiment = spice_entity::experiments::Entity::find_by_id(experiment_id)
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

#[cfg(test)]
mod tests {
    use crate::config::test_helpers::setup_test_app;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tower::ServiceExt;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_excel_upload_endpoint_exists() {
        let app = setup_test_app().await;

        // Create an experiment first
        let experiment_data = json!({
            "name": "Excel Upload Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test Excel upload endpoint"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/experiments")
                    .header("content-type", "application/json")
                    .body(Body::from(experiment_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response.status().is_success(),
            "Failed to create test experiment"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let experiment_id = experiment["id"].as_str().unwrap();

        // Test that the Excel upload endpoint exists by sending a request without file
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/experiments/{experiment_id}/process-excel"))
                    .header("content-type", "multipart/form-data")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should not be 404 - endpoint should exist
        assert_ne!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Excel upload endpoint should exist"
        );

        // Should be 400 (bad request) because no file was uploaded
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Should return bad request for no file"
        );

        println!("Excel upload endpoint status: {}", response.status());
    }

    #[tokio::test]
    async fn test_excel_upload_invalid_file_type() {
        let app = setup_test_app().await;

        // Create an experiment first
        let experiment_data = json!({
            "name": "Excel Invalid File Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test invalid file type"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/experiments")
                    .header("content-type", "application/json")
                    .body(Body::from(experiment_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let experiment_id = experiment["id"].as_str().unwrap();

        // Create a simple multipart request with a text file
        let boundary = "----formdata-boundary";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"excel_file\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\nThis is not an Excel file\r\n--{boundary}--\r\n"
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/experiments/{experiment_id}/process-excel"))
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Should reject non-Excel files"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_text = String::from_utf8_lossy(&body_bytes);
        assert!(
            response_text.contains("Excel file"),
            "Error should mention Excel file requirement"
        );
    }

    #[tokio::test]
    async fn test_excel_upload_experiment_not_found() {
        let app = setup_test_app().await;

        let fake_experiment_id = "00000000-0000-0000-0000-000000000000";

        // Create a simple multipart request with an Excel file
        let boundary = "----formdata-boundary";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"excel_file\"; filename=\"test.xlsx\"\r\nContent-Type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet\r\n\r\nFake Excel content\r\n--{boundary}--\r\n",
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/experiments/{fake_experiment_id}/process-excel"
                    ))
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Should return 404 for non-existent experiment"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_text = String::from_utf8_lossy(&body_bytes);
        assert!(
            response_text.contains("Experiment not found"),
            "Error should mention experiment not found"
        );
    }

    #[tokio::test]
    async fn test_processing_status_endpoint() {
        let app = setup_test_app().await;

        // Create an experiment first
        let experiment_data = json!({
            "name": "Processing Status Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test processing status"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/experiments")
                    .header("content-type", "application/json")
                    .body(Body::from(experiment_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let experiment_id = experiment["id"].as_str().unwrap();

        let job_id = Uuid::new_v4();

        // Test processing status endpoint
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/experiments/{experiment_id}/process-status/{job_id}",
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Processing status endpoint should work"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let status_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(status_response["experiment_id"], experiment_id);
        assert_eq!(status_response["job_id"], job_id.to_string());
        assert!(
            status_response["status"].is_string(),
            "Should have status field"
        );
    }
}
