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
            "Failed to create test experiment: {:?}",
            body
        );
        uuid::Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
    }

    // #[tokio::test]
    // async fn test_asset_crud_operations() {
    //     let db = setup_test_db().await;
    //     let app = setup_test_app().await;
    //     cleanup_test_data(&db).await;

    //     // Create a test experiment for the asset
    //     let experiment_id = create_test_experiment(&app, "ASSET_CRUD").await;

    //     // Test creating an asset with unique S3 key
    //     let unique_id = uuid::Uuid::new_v4();
    //     let asset_data = json!({
    //         "experiment_id": experiment_id,
    //         "original_filename": format!("test_data_{}.csv", unique_id),
    //         "s3_key": format!("test/data/test_data_{}.csv", unique_id),
    //         "size_bytes": 1024,
    //         "uploaded_by": "test@example.com",
    //         "uploaded_at": "2024-06-20T15:00:00Z",
    //         "is_deleted": false,
    //         "type": "data",
    //         "role": "results"
    //     });

    //     let response = app
    //         .clone()
    //         .oneshot(
    //             Request::builder()
    //                 .method("POST")
    //                 .uri("/api/assets")
    //                 .header("content-type", "application/json")
    //                 .body(Body::from(asset_data.to_string()))
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     let (status, body) = extract_response_body(response).await;
    //     assert_eq!(
    //         status,
    //         StatusCode::CREATED,
    //         "Failed to create asset: {:?}",
    //         body
    //     );

    //     // Validate response structure
    //     assert!(body["id"].is_string(), "Response should include ID");
    //     assert!(body["original_filename"].is_string());
    //     assert_eq!(body["type"], "data");
    //     assert_eq!(body["role"], "results");
    //     assert!(body["created_at"].is_string());

    //     let asset_id = body["id"].as_str().unwrap();

    //     // Test getting the asset by ID
    //     let get_response = app
    //         .clone()
    //         .oneshot(
    //             Request::builder()
    //                 .method("GET")
    //                 .uri(&format!("/api/assets/{}", asset_id))
    //                 .body(Body::empty())
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     let (get_status, get_body) = extract_response_body(get_response).await;
    //     assert_eq!(
    //         get_status,
    //         StatusCode::OK,
    //         "Failed to get asset: {:?}",
    //         get_body
    //     );
    //     assert_eq!(get_body["id"], asset_id);

    //     // Test getting all assets
    //     let list_response = app
    //         .clone()
    //         .oneshot(
    //             Request::builder()
    //                 .method("GET")
    //                 .uri("/api/assets")
    //                 .body(Body::empty())
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     let (list_status, list_body) = extract_response_body(list_response).await;
    //     assert_eq!(list_status, StatusCode::OK, "Failed to get assets");
    //     assert!(list_body["items"].is_array());

    //     cleanup_test_data(&db).await;
    // }

    // #[tokio::test]
    // async fn test_asset_filtering() {
    //     let db = setup_test_db().await;
    //     let app = setup_test_app().await;
    //     cleanup_test_data(&db).await;

    //     // Create a test experiment for the assets
    //     let experiment_id = create_test_experiment(&app, "ASSET_FILTERING").await;

    //     // Create test assets for filtering
    //     let asset_types = ["data", "image"];
    //     for asset_type in asset_types {
    //         let unique_id = uuid::Uuid::new_v4();
    //         let asset_data = json!({
    //             "experiment_id": experiment_id,
    //             "original_filename": format!("filter_test_{}_{}.file", asset_type, unique_id),
    //             "s3_key": format!("test/{}/filter_test_{}.file", asset_type, unique_id),
    //             "type": asset_type,
    //             "size_bytes": 100,
    //             "uploaded_at": "2024-06-20T15:00:00Z",
    //             "is_deleted": false,
    //             "role": "results"
    //         });

    //         let response = app
    //             .clone()
    //             .oneshot(
    //                 Request::builder()
    //                     .method("POST")
    //                     .uri("/api/assets")
    //                     .header("content-type", "application/json")
    //                     .body(Body::from(asset_data.to_string()))
    //                     .unwrap(),
    //             )
    //             .await
    //             .unwrap();

    //         let (status, body) = extract_response_body(response).await;
    //         assert_eq!(
    //             status,
    //             StatusCode::CREATED,
    //             "Failed to create asset for filtering test: {:?}",
    //             body
    //         );
    //     }

    //     // Test filtering by type
    //     let filter_response = app
    //         .clone()
    //         .oneshot(
    //             Request::builder()
    //                 .method("GET")
    //                 .uri("/api/assets?filter[type]=data")
    //                 .body(Body::empty())
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     let (filter_status, filter_body) = extract_response_body(filter_response).await;
    //     assert_eq!(
    //         filter_status,
    //         StatusCode::OK,
    //         "Type filtering should work: {:?}",
    //         filter_body
    //     );

    //     // Test filtering by role
    //     let role_filter_response = app
    //         .clone()
    //         .oneshot(
    //             Request::builder()
    //                 .method("GET")
    //                 .uri("/api/assets?filter[role]=results")
    //                 .body(Body::empty())
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     let (role_filter_status, _) = extract_response_body(role_filter_response).await;
    //     assert_eq!(
    //         role_filter_status,
    //         StatusCode::OK,
    //         "Role filtering should work"
    //     );

    //     cleanup_test_data(&db).await;
    // }

    // #[tokio::test]
    // async fn test_asset_type_validation() {
    //     let db = setup_test_db().await;
    //     let app = setup_test_app().await;
    //     cleanup_test_data(&db).await;

    //     // Create a test experiment for the assets
    //     let experiment_id = create_test_experiment(&app, "ASSET_TYPE_VALIDATION").await;

    //     // Test valid asset types (these are just strings, not enums)
    //     for asset_type in ["data", "image", "log", "report", "protocol"] {
    //         let unique_id = uuid::Uuid::new_v4();
    //         let asset_data = json!({
    //             "experiment_id": experiment_id,
    //             "original_filename": format!("test_{}_{}.file", asset_type, unique_id),
    //             "s3_key": format!("test/{}/test_{}.file", asset_type, unique_id),
    //             "type": asset_type,
    //             "size_bytes": 100,
    //             "uploaded_at": "2024-06-20T15:00:00Z",
    //             "is_deleted": false
    //         });

    //         let response = app
    //             .clone()
    //             .oneshot(
    //                 Request::builder()
    //                     .method("POST")
    //                     .uri("/api/assets")
    //                     .header("content-type", "application/json")
    //                     .body(Body::from(asset_data.to_string()))
    //                     .unwrap(),
    //             )
    //             .await
    //             .unwrap();

    //         let (status, body) = extract_response_body(response).await;
    //         assert_eq!(
    //             status,
    //             StatusCode::CREATED,
    //             "Valid asset type {} should be accepted. Body: {:?}",
    //             asset_type,
    //             body
    //         );
    //     }

    //     // Test with missing required fields
    //     let incomplete_data = json!({
    //         "original_filename": "incomplete.file",
    //         // Missing s3_key, type, etc.
    //     });

    //     let response = app
    //         .clone()
    //         .oneshot(
    //             Request::builder()
    //                 .method("POST")
    //                 .uri("/api/assets")
    //                 .header("content-type", "application/json")
    //                 .body(Body::from(incomplete_data.to_string()))
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     let (status, _body) = extract_response_body(response).await;
    //     assert!(
    //         status.is_client_error(),
    //         "Should reject incomplete asset data"
    //     );

    //     cleanup_test_data(&db).await;
    // }
}
