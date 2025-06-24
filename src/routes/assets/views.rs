use super::models::{Asset, AssetCreate, AssetUpdate};
use crate::common::auth::Role;
use crate::common::state::AppState;
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::{CRUDResource, crud_handlers};
use utoipa_axum::{router::OpenApiRouter, routes};

crud_handlers!(Asset, AssetUpdate, AssetCreate);

pub fn router(state: &AppState) -> OpenApiRouter
where
    Asset: CRUDResource,
{
    let mut mutating_router = OpenApiRouter::new()
        .routes(routes!(get_one_handler))
        .routes(routes!(get_all_handler))
        .routes(routes!(create_one_handler))
        .routes(routes!(update_one_handler))
        .routes(routes!(delete_one_handler))
        .routes(routes!(delete_many_handler))
        .with_state(state.db.clone());

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

#[cfg(test)]
mod tests {
    use crate::config::test_helpers::{cleanup_test_data, setup_test_app, setup_test_db};
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
    use tower::ServiceExt;

    async fn extract_response_body(response: axum::response::Response) -> (StatusCode, Value) {
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");
        let body: Value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| json!({"error": "Invalid JSON response"}));
        (status, body)
    }

    async fn create_test_experiment(app: &axum::Router, test_suffix: &str) -> uuid::Uuid {
        let experiment_data = json!({
            "name": format!("Test Experiment {}", test_suffix),
            "username": "test_user",
            "performed_at": "2024-06-20T12:00:00Z",
            "temperature_ramp": "1.0",
            "temperature_start": "-20.0",
            "temperature_end": "0.0",
            "is_calibration": false,
            "remarks": "Test experiment for asset tests"
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

        let (status, body) = extract_response_body(response).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Failed to create test experiment: {body:?}"
        );
        uuid::Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
    }

    #[tokio::test]
    async fn test_s3_asset_crud_operations() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Create a test experiment for the asset
        let experiment_id = create_test_experiment(&app, "ASSET_CRUD").await;

        // Test creating an asset
        let asset_data = json!({
            "experiment_id": experiment_id,
            "original_filename": "test_file.csv",
            "s3_key": format!("test-files/{}", uuid::Uuid::new_v4()),
            "type": "data",
            "role": "input",
            "size_bytes": 1024,
            "uploaded_by": "test_user"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/assets")
                    .header("content-type", "application/json")
                    .body(Body::from(asset_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body) = extract_response_body(response).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Failed to create asset: {body:?}"
        );

        let asset_id = body["id"].as_str().unwrap();

        // Test reading the created asset
        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/assets/{asset_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (get_status, get_body) = extract_response_body(get_response).await;
        assert_eq!(get_status, StatusCode::OK, "Failed to get asset");
        assert_eq!(get_body["id"], asset_id);
        assert_eq!(get_body["original_filename"], "test_file.csv");

        // Test updating the asset
        let update_data = json!({
            "experiment_id": experiment_id,
            "original_filename": "updated_file.csv",
            "s3_key": get_body["s3_key"], // Keep same S3 key
            "type": "data",
            "role": "output",
            "size_bytes": 2048,
            "uploaded_by": "test_user"
        });

        let update_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/assets/{asset_id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(update_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (update_status, update_body) = extract_response_body(update_response).await;
        assert_eq!(
            update_status,
            StatusCode::OK,
            "Failed to update asset: {update_body:?}"
        );
        assert_eq!(update_body["original_filename"], "updated_file.csv");
        assert_eq!(update_body["role"], "output");

        // Test deleting the asset
        let delete_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/assets/{asset_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (delete_status, _) = extract_response_body(delete_response).await;
        assert_eq!(delete_status, StatusCode::NO_CONTENT);

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_s3_asset_validation() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Test creating asset with invalid data (null filename)
        let invalid_data = json!({
            "original_filename": null,
            "s3_key": "test/key",
            "type": "data"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/assets")
                    .header("content-type", "application/json")
                    .body(Body::from(invalid_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, _body) = extract_response_body(response).await;
        assert!(
            status.is_client_error(),
            "Should reject asset with null filename"
        );

        // Test creating asset with missing required fields
        let incomplete_data = json!({
            "s3_key": "test/key"
            // Missing original_filename and type
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/assets")
                    .header("content-type", "application/json")
                    .body(Body::from(incomplete_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, _body) = extract_response_body(response).await;
        assert!(
            status.is_client_error(),
            "Should reject incomplete asset data"
        );

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_s3_asset_filtering() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Create a test experiment for the assets
        let experiment_id = create_test_experiment(&app, "ASSET_FILTERING").await;

        // Create some test assets with different types
        let asset_types = ["data", "image", "document"];
        for (i, asset_type) in asset_types.iter().enumerate() {
            let asset_data = json!({
                "experiment_id": experiment_id,
                "original_filename": format!("test_file_{}.txt", i),
                "s3_key": format!("test-files/{}-{}", asset_type, uuid::Uuid::new_v4()),
                "type": asset_type,
                "size_bytes": 1024 * (i + 1),
                "uploaded_by": "test_user"
            });

            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/assets")
                        .header("content-type", "application/json")
                        .body(Body::from(asset_data.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();

            let (status, _) = extract_response_body(response).await;
            assert_eq!(status, StatusCode::CREATED);
        }

        // Test filtering by experiment_id
        let filter_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/assets?filter[experiment_id]={experiment_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (filter_status, filter_body) = extract_response_body(filter_response).await;
        assert_eq!(
            filter_status,
            StatusCode::OK,
            "Failed to filter assets by experiment_id"
        );
        let items = filter_body["items"].as_array().unwrap();
        assert!(items.len() >= 3, "Should find at least 3 assets");

        // Test filtering by type
        let type_filter_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/assets?filter[type]=data")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (type_status, type_body) = extract_response_body(type_filter_response).await;
        assert_eq!(
            type_status,
            StatusCode::OK,
            "Failed to filter assets by type"
        );
        let filtered_items = type_body["items"].as_array().unwrap();
        for item in filtered_items {
            assert_eq!(item["type"], "data");
        }

        cleanup_test_data(&db).await;
    }
}
