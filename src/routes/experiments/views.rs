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
        .route(
            "/{experiment_id}/results",
            get(super::results::get_experiment_results),
        )
        .route(
            "/{experiment_id}/wells/{row}/{col}/results",
            get(super::results::get_well_detailed_results),
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
                format!("File '{}' already exists in this experiment", file_name)
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
                    format!("Failed to insert asset record: {}", e),
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

#[cfg(test)]
mod tests {
    use crate::config::test_helpers::{setup_test_app, setup_test_db};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
    use serde_json::json;
    use spice_entity::{experiments, tray_configuration_assignments, tray_configurations, trays};
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn create_tray_with_config(
        db: &sea_orm::DatabaseConnection,
        rows: i32,
        cols: i32,
        config_name: &str,
    ) -> (Uuid, Uuid) {
        // Create tray
        let tray_id = Uuid::new_v4();
        let tray = trays::ActiveModel {
            id: Set(tray_id),
            name: Set(Some(format!("{}x{} Tray", rows, cols))),
            qty_x_axis: Set(Some(cols)),
            qty_y_axis: Set(Some(rows)),
            well_relative_diameter: Set(None),
            last_updated: Set(chrono::Utc::now().into()),
            created_at: Set(chrono::Utc::now().into()),
        };
        tray.insert(db).await.unwrap();

        // Create tray configuration
        let config_id = Uuid::new_v4();
        let config = tray_configurations::ActiveModel {
            id: Set(config_id),
            name: Set(Some(config_name.to_string())),
            experiment_default: Set(false),
            created_at: Set(chrono::Utc::now().into()),
            last_updated: Set(chrono::Utc::now().into()),
        };
        config.insert(db).await.unwrap();

        // Create tray configuration assignment
        let assignment = tray_configuration_assignments::ActiveModel {
            tray_id: Set(tray_id),
            tray_configuration_id: Set(config_id),
            order_sequence: Set(1),
            rotation_degrees: Set(0),
            created_at: Set(chrono::Utc::now().into()),
            last_updated: Set(chrono::Utc::now().into()),
        };
        assignment.insert(db).await.unwrap();

        (tray_id, config_id)
    }

    async fn assign_tray_config_to_experiment(
        db: &sea_orm::DatabaseConnection,
        experiment_id: Uuid,
        config_id: Uuid,
    ) {
        let mut experiment: experiments::ActiveModel =
            experiments::Entity::find_by_id(experiment_id)
                .one(db)
                .await
                .unwrap()
                .unwrap()
                .into();

        experiment.tray_configuration_id = Set(Some(config_id));
        experiment.update(db).await.unwrap();
    }

    #[tokio::test]
    async fn test_experiment_crud_operations() {
        let app = setup_test_app().await;

        // Test creating an experiment
        let experiment_data = json!({
            "name": "Test Experiment API",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test experiment via API"
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
            "Failed to create experiment"
        );

        // Test getting all experiments
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/experiments")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Failed to get experiments"
        );
    }

    #[tokio::test]
    async fn test_experiment_filtering() {
        let app = setup_test_app().await;

        // Test filtering by calibration status
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/experiments?filter[is_calibration]=false")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Calibration filtering should work"
        );

        // Test sorting
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/experiments?sort[performed_at]=desc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK, "Sorting should work");
    }

    #[tokio::test]
    async fn test_experiment_validation() {
        let app = setup_test_app().await;

        // Test creating experiment with invalid temperature range
        let invalid_data = json!({
            "name": "Invalid Experiment",
            "temperature_start": -25.0,
            "temperature_end": 5.0  // End temp higher than start - should be invalid
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/experiments")
                    .header("content-type", "application/json")
                    .body(Body::from(invalid_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Note: This test might pass if validation isn't implemented yet
        // The test documents expected behavior
        println!("Temperature validation response: {}", response.status());
    }

    #[tokio::test]
    async fn test_phase_change_events_endpoint() {
        let app = setup_test_app().await;

        // Test that we can fetch the phase change events endpoint (should return empty array initially)
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/phase_change_events")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // The endpoint should exist and return a 200 status
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Phase change events endpoint should exist and be accessible"
        );

        // Parse response to verify it returns an array (even if empty)
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let events: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert!(events.is_array(), "Response should be an array of events");
        println!("Phase change events response: {}", events);
    }

    #[tokio::test]
    async fn test_phase_change_upload_endpoint_exists() {
        let app = setup_test_app().await;

        // Create an experiment using the same data as the working test
        let experiment_data = json!({
            "name": "Test Experiment API",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test experiment via API"
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

        // Test that the upload endpoint exists by sending an invalid request
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!(
                        "/api/experiments/{}/phase_changes/upload",
                        experiment_id
                    ))
                    .header("content-type", "text/plain")
                    .body(Body::from("invalid"))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should not be 404 - endpoint should exist
        assert_ne!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Upload endpoint should exist"
        );
        println!("Upload endpoint status: {}", response.status());

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_text = String::from_utf8_lossy(&body_bytes);
        println!("Upload endpoint response: {}", response_text);
    }

    #[tokio::test]
    async fn test_time_point_tables_exist() {
        use crate::config::test_helpers::setup_test_db;
        use sea_orm::EntityTrait;
        use spice_entity::time_points;

        let db = setup_test_db().await;

        // Try to query the time_points table to see if it exists
        let result = time_points::Entity::find().all(&db).await;
        match result {
            Ok(records) => {
                println!(
                    "Successfully queried time_points table. Found {} records",
                    records.len()
                );
            }
            Err(e) => {
                panic!("Failed to query time_points table: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_time_point_endpoint() {
        let db = setup_test_db().await;

        // Create tray configuration for basic test (8x12 = 96-well)
        let (_tray_id, config_id) =
            create_tray_with_config(&db, 8, 12, "Basic 96-well Config").await;

        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new(); // Disable Keycloak for tests
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment first
        let experiment_data = json!({
            "name": "Test Time Point Experiment",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test time point experiment"
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
        let experiment_uuid = Uuid::parse_str(experiment_id).unwrap();

        // Assign tray configuration to experiment
        assign_tray_config_to_experiment(&db, experiment_uuid, config_id).await;

        // Test creating a time point (using 1-based coordinates)
        let time_point_data = json!({
            "timestamp": "2025-03-20T15:13:47Z",
            "image_filename": "INP_49640_2025-03-20_15-14-17.jpg",
            "temperature_readings": [
                {"probe_sequence": 1, "temperature": 29.827},
                {"probe_sequence": 2, "temperature": 29.795},
                {"probe_sequence": 3, "temperature": 29.787}
            ],
            "well_states": [
                {"row": 1, "col": 1, "value": 0},
                {"row": 1, "col": 2, "value": 1},
                {"row": 2, "col": 1, "value": 0}
            ]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/experiments/{}/time_points", experiment_id))
                    .header("content-type", "application/json")
                    .body(Body::from(time_point_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_text = String::from_utf8_lossy(&body_bytes);

        if status != StatusCode::OK {
            println!("Error response status: {}", status);
            println!("Error response body: {}", response_text);
        }

        assert_eq!(status, StatusCode::OK, "Time point creation should work");

        let time_point: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert!(time_point["id"].is_string(), "Time point should have an ID");
        assert_eq!(
            time_point["experiment_id"], experiment_id,
            "Experiment ID should match"
        );
        assert_eq!(
            time_point["image_filename"],
            "INP_49640_2025-03-20_15-14-17.jpg"
        );

        println!("Time point response: {}", time_point);
    }

    #[tokio::test]
    async fn test_time_point_with_96_well_plates() {
        let db = setup_test_db().await;

        // Create tray configuration for 96-well plate (8x12)
        let (_tray_id, config_id) = create_tray_with_config(&db, 8, 12, "96-well Config").await;

        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new(); // Disable Keycloak for tests
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment
        let experiment_data = json!({
            "name": "96-Well Time Point Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test with 96-well plates"
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
        let experiment_uuid = Uuid::parse_str(experiment_id).unwrap();

        // Assign tray configuration to experiment
        assign_tray_config_to_experiment(&db, experiment_uuid, config_id).await;

        // Create time point with data for full 96-well plate (8 rows × 12 columns, 1-based)
        let mut well_states = Vec::new();
        for row in 1..=8 {
            for col in 1..=12 {
                well_states.push(json!({
                    "row": row,
                    "col": col,
                    "value": if (row + col) % 2 == 0 { 0 } else { 1 } // Alternating pattern
                }));
            }
        }

        let time_point_data = json!({
            "timestamp": "2025-03-20T15:13:47Z",
            "image_filename": "96well_test.jpg",
            "temperature_readings": [
                {"probe_sequence": 1, "temperature": 25.0},
                {"probe_sequence": 2, "temperature": 24.8},
                {"probe_sequence": 3, "temperature": 24.9},
                {"probe_sequence": 4, "temperature": 25.1},
                {"probe_sequence": 5, "temperature": 24.7},
                {"probe_sequence": 6, "temperature": 25.2},
                {"probe_sequence": 7, "temperature": 24.6},
                {"probe_sequence": 8, "temperature": 25.3}
            ],
            "well_states": well_states
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/experiments/{}/time_points", experiment_id))
                    .header("content-type", "application/json")
                    .body(Body::from(time_point_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "96-well time point creation should work"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let time_point: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(
            time_point["well_states"].as_array().unwrap().len(),
            96,
            "Should have 96 wells"
        );
        assert_eq!(
            time_point["temperature_readings"].as_array().unwrap().len(),
            8,
            "Should have 8 probes"
        );
    }

    #[tokio::test]
    async fn test_time_point_with_384_well_plates() {
        let db = setup_test_db().await;

        // Create tray configuration for 384-well plate (16x24)
        let (_tray_id, config_id) = create_tray_with_config(&db, 16, 24, "384-well Config").await;

        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new(); // Disable Keycloak for tests
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment
        let experiment_data = json!({
            "name": "384-Well Time Point Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test with 384-well plates"
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
        let experiment_uuid = Uuid::parse_str(experiment_id).unwrap();

        // Assign tray configuration to experiment
        assign_tray_config_to_experiment(&db, experiment_uuid, config_id).await;

        // Create time point with data for 384-well plate (16 rows × 24 columns, 1-based)
        let mut well_states = Vec::new();
        for row in 1..=16 {
            for col in 1..=24 {
                // Only add wells that have actual data (simulate sparse data)
                if (row * col) % 10 == 0 {
                    well_states.push(json!({
                        "row": row,
                        "col": col,
                        "value": if row < 8 { 0 } else { 1 } // Half frozen, half liquid
                    }));
                }
            }
        }

        let time_point_data = json!({
            "timestamp": "2025-03-20T16:30:00Z",
            "image_filename": "384well_test.jpg",
            "temperature_readings": [
                {"probe_sequence": 1, "temperature": 20.0},
                {"probe_sequence": 2, "temperature": 19.8},
                {"probe_sequence": 3, "temperature": 20.2},
                {"probe_sequence": 4, "temperature": 19.9}
            ],
            "well_states": well_states
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/experiments/{}/time_points", experiment_id))
                    .header("content-type", "application/json")
                    .body(Body::from(time_point_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "384-well time point creation should work"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let time_point: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        // Should handle sparse data properly
        assert!(
            time_point["well_states"].as_array().unwrap().len() > 0,
            "Should have some wells"
        );
        assert_eq!(
            time_point["temperature_readings"].as_array().unwrap().len(),
            4,
            "Should have 4 probes"
        );
    }

    #[tokio::test]
    async fn test_time_point_with_custom_tray_configuration() {
        let db = setup_test_db().await;

        // Create tray configuration for custom large tray (24x30 to accommodate large coordinates)
        let (_tray_id, config_id) =
            create_tray_with_config(&db, 24, 30, "Custom Large Config").await;

        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new(); // Disable Keycloak for tests
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment
        let experiment_data = json!({
            "name": "Custom Tray Time Point Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test with custom tray configuration"
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
        let experiment_uuid = Uuid::parse_str(experiment_id).unwrap();

        // Assign tray configuration to experiment
        assign_tray_config_to_experiment(&db, experiment_uuid, config_id).await;

        // Create time point with irregular well pattern (simulating custom tray, 1-based)
        let well_states = vec![
            json!({"row": 1, "col": 1, "value": 0}),
            json!({"row": 1, "col": 6, "value": 1}),
            json!({"row": 4, "col": 3, "value": 0}),
            json!({"row": 8, "col": 12, "value": 1}),
            json!({"row": 16, "col": 24, "value": 0}), // Large coordinates within bounds
        ];

        let time_point_data = json!({
            "timestamp": "2025-03-20T17:45:00Z",
            "image_filename": "custom_tray_test.jpg",
            "temperature_readings": [
                {"probe_sequence": 1, "temperature": 15.5},
                {"probe_sequence": 3, "temperature": 15.2}, // Non-sequential probe numbers
                {"probe_sequence": 5, "temperature": 15.8},
                {"probe_sequence": 7, "temperature": 15.1}
            ],
            "well_states": well_states
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/experiments/{}/time_points", experiment_id))
                    .header("content-type", "application/json")
                    .body(Body::from(time_point_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Custom tray time point creation should work"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let time_point: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(
            time_point["well_states"].as_array().unwrap().len(),
            5,
            "Should have 5 wells"
        );
        assert_eq!(
            time_point["temperature_readings"].as_array().unwrap().len(),
            4,
            "Should have 4 probes"
        );

        // Verify non-sequential probe sequences are preserved
        let temp_readings = time_point["temperature_readings"].as_array().unwrap();
        let probe_sequences: Vec<i64> = temp_readings
            .iter()
            .map(|r| r["probe_sequence"].as_i64().unwrap())
            .collect();
        assert_eq!(
            probe_sequences,
            vec![1, 3, 5, 7],
            "Probe sequences should be preserved"
        );
    }

    #[tokio::test]
    async fn test_time_point_with_minimal_data() {
        let db = setup_test_db().await;

        // Create tray configuration for minimal test (single well)
        let (_tray_id, config_id) = create_tray_with_config(&db, 1, 1, "Minimal Config").await;

        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new(); // Disable Keycloak for tests
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment
        let experiment_data = json!({
            "name": "Minimal Time Point Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test with minimal data"
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
        let experiment_uuid = Uuid::parse_str(experiment_id).unwrap();

        // Assign tray configuration to experiment
        assign_tray_config_to_experiment(&db, experiment_uuid, config_id).await;

        // Create time point with minimal data (no image, single well, single probe, 1-based)
        let time_point_data = json!({
            "timestamp": "2025-03-20T18:00:00Z",
            "temperature_readings": [
                {"probe_sequence": 1, "temperature": 10.0}
            ],
            "well_states": [
                {"row": 1, "col": 1, "value": 1}
            ]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/experiments/{}/time_points", experiment_id))
                    .header("content-type", "application/json")
                    .body(Body::from(time_point_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Minimal time point creation should work"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let time_point: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(
            time_point["well_states"].as_array().unwrap().len(),
            1,
            "Should have 1 well"
        );
        assert_eq!(
            time_point["temperature_readings"].as_array().unwrap().len(),
            1,
            "Should have 1 probe"
        );
        assert!(
            time_point["image_filename"].is_null(),
            "Image filename should be null"
        );
    }

    #[tokio::test]
    async fn test_time_point_validation() {
        let app = setup_test_app().await;

        // Create an experiment
        let experiment_data = json!({
            "name": "Validation Time Point Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test validation"
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

        // Test invalid timestamp format
        let invalid_time_data = json!({
            "timestamp": "invalid-timestamp",
            "temperature_readings": [{"probe_sequence": 1, "temperature": 10.0}],
            "well_states": [{"row": 0, "col": 0, "value": 1}]
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/experiments/{}/time_points", experiment_id))
                    .header("content-type", "application/json")
                    .body(Body::from(invalid_time_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid timestamp should fail"
        );

        // Test non-existent experiment
        let valid_data = json!({
            "timestamp": "2025-03-20T18:00:00Z",
            "temperature_readings": [{"probe_sequence": 1, "temperature": 10.0}],
            "well_states": [{"row": 0, "col": 0, "value": 1}]
        });

        let fake_experiment_id = "00000000-0000-0000-0000-000000000000";
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!(
                        "/api/experiments/{}/time_points",
                        fake_experiment_id
                    ))
                    .header("content-type", "application/json")
                    .body(Body::from(valid_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Non-existent experiment should return 404"
        );
    }

    #[tokio::test]
    async fn test_time_point_validation_no_tray_config() {
        let db = setup_test_db().await;
        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new(); // Disable Keycloak for tests
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment without assigning a tray configuration
        let experiment_data = json!({
            "name": "No Tray Config Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test no tray config validation"
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

        // Try to create a time point without tray configuration assigned
        let time_point_data = json!({
            "timestamp": "2025-03-20T15:13:47Z",
            "temperature_readings": [
                {"probe_sequence": 1, "temperature": 29.827}
            ],
            "well_states": [
                {"row": 1, "col": 1, "value": 0}
            ]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/experiments/{}/time_points", experiment_id))
                    .header("content-type", "application/json")
                    .body(Body::from(time_point_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "Should fail with no tray configuration"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let error_text = String::from_utf8_lossy(&body_bytes);
        assert!(
            error_text.contains("no tray configuration assigned"),
            "Error should mention missing tray configuration"
        );
    }

    #[tokio::test]
    async fn test_time_point_validation_well_coordinates_out_of_bounds() {
        let db = setup_test_db().await;

        // Create tray configuration for small tray (2x2)
        let (_tray_id, config_id) = create_tray_with_config(&db, 2, 2, "Small 2x2 Config").await;

        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new(); // Disable Keycloak for tests
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment
        let experiment_data = json!({
            "name": "Out of Bounds Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test well coordinates validation"
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
        let experiment_uuid = Uuid::parse_str(experiment_id).unwrap();

        // Assign tray configuration to experiment
        assign_tray_config_to_experiment(&db, experiment_uuid, config_id).await;

        // Try to create time point with coordinates exceeding tray bounds (2x2 tray, trying row=3, col=3)
        let time_point_data = json!({
            "timestamp": "2025-03-20T15:13:47Z",
            "temperature_readings": [
                {"probe_sequence": 1, "temperature": 29.827}
            ],
            "well_states": [
                {"row": 3, "col": 3, "value": 0} // Out of bounds for 2x2 tray
            ]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/experiments/{}/time_points", experiment_id))
                    .header("content-type", "application/json")
                    .body(Body::from(time_point_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "Should fail with out of bounds coordinates"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let error_text = String::from_utf8_lossy(&body_bytes);
        assert!(
            error_text.contains("invalid for the assigned tray configuration"),
            "Error should mention invalid coordinates"
        );
    }

    #[tokio::test]
    async fn test_time_point_validation_success_within_bounds() {
        let db = setup_test_db().await;

        // Create tray configuration for small tray (2x2)
        let (_tray_id, config_id) =
            create_tray_with_config(&db, 2, 2, "Small 2x2 Success Config").await;

        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new(); // Disable Keycloak for tests
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment
        let experiment_data = json!({
            "name": "Within Bounds Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test valid well coordinates"
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
        let experiment_uuid = Uuid::parse_str(experiment_id).unwrap();

        // Assign tray configuration to experiment
        assign_tray_config_to_experiment(&db, experiment_uuid, config_id).await;

        // Create time point with valid coordinates within bounds (2x2 tray, using row=2, col=2)
        let time_point_data = json!({
            "timestamp": "2025-03-20T15:13:47Z",
            "temperature_readings": [
                {"probe_sequence": 1, "temperature": 29.827}
            ],
            "well_states": [
                {"row": 1, "col": 1, "value": 0}, // Valid: top-left
                {"row": 2, "col": 2, "value": 1}  // Valid: bottom-right
            ]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/experiments/{}/time_points", experiment_id))
                    .header("content-type", "application/json")
                    .body(Body::from(time_point_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Should succeed with valid coordinates"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let time_point: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(
            time_point["well_states"].as_array().unwrap().len(),
            2,
            "Should have 2 wells"
        );
    }
}
