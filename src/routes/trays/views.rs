use super::models::{TrayConfiguration, TrayConfigurationCreate, TrayConfigurationUpdate};
use crate::common::auth::Role;
use crate::common::state::AppState;
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::{CRUDResource, crud_handlers};
use utoipa_axum::{router::OpenApiRouter, routes};

crud_handlers!(
    TrayConfiguration,
    TrayConfigurationUpdate,
    TrayConfigurationCreate
);

pub fn router(state: &AppState) -> OpenApiRouter
where
    TrayConfiguration: CRUDResource,
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
            TrayConfiguration::RESOURCE_NAME_PLURAL
        );
    }

    mutating_router
}

// #[cfg(test)]
// mod tests {
//     use crate::config::test_helpers::{cleanup_test_data, setup_test_app, setup_test_db};
//     use axum::body::{Body, to_bytes};
//     use axum::http::{Request, StatusCode};
//     use serde_json::{Value, json};
//     use tower::ServiceExt;

//     async fn extract_response_body(response: axum::response::Response) -> (StatusCode, Value) {
//         let status = response.status();
//         let bytes = to_bytes(response.into_body(), usize::MAX)
//             .await
//             .expect("Failed to read response body");
//         let body: Value = serde_json::from_slice(&bytes)
//             .unwrap_or_else(|_| json!({"error": "Invalid JSON response"}));
//         (status, body)
//     }

//     #[tokio::test]
//     async fn test_tray_configuration_create_complete() {
//         let db = setup_test_db().await;
//         let app = setup_test_app().await;
//         cleanup_test_data(&db).await;

//         // Test creating a comprehensive tray configuration
//         let tray_config_input = json!({
//             "name": "Standard 96-well Configuration",
//             "experiment_default": true,
//             "trays": [
//                 {
//                     "order_sequence": 1,
//                     "rotation_degrees": 0,
//                     "trays": [
//                         {
//                             "name": "96-well plate A",
//                             "qty_x_axis": 12,
//                             "qty_y_axis": 8,
//                             "well_relative_diameter": "6.35"
//                         }
//                     ]
//                 },
//                 {
//                     "order_sequence": 2,
//                     "rotation_degrees": 90,
//                     "trays": [
//                         {
//                             "name": "96-well plate B",
//                             "qty_x_axis": 12,
//                             "qty_y_axis": 8,
//                             "well_relative_diameter": "6.35"
//                         }
//                     ]
//                 }
//             ]
//         });

//         let response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("POST")
//                     .uri("/api/trays")
//                     .header("content-type", "application/json")
//                     .body(Body::from(tray_config_input.to_string()))
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (status, body) = extract_response_body(response).await;
//         assert_eq!(
//             status,
//             StatusCode::CREATED,
//             "Failed to create tray configuration: {:?}",
//             body
//         );

//         // Validate response structure
//         assert!(body["id"].is_string(), "Response should include ID");
//         assert_eq!(body["name"], "Standard 96-well Configuration");
//         assert_eq!(body["experiment_default"], true);
//         assert!(body["created_at"].is_string());
//         assert!(body["last_updated"].is_string());
//         assert!(body["trays"].is_array());
//         assert_eq!(body["trays"].as_array().unwrap().len(), 2);

//         // Validate tray assignments structure
//         let first_tray = &body["trays"][0];
//         assert_eq!(first_tray["order_sequence"], 1);
//         assert_eq!(first_tray["rotation_degrees"], 0);
//         assert!(first_tray["trays"].is_array());

//         let nested_tray = &first_tray["trays"][0];
//         assert_eq!(nested_tray["name"], "96-well plate A");
//         assert_eq!(nested_tray["qty_x_axis"], 12);
//         assert_eq!(nested_tray["qty_y_axis"], 8);

//         cleanup_test_data(&db).await;
//     }

//     #[tokio::test]
//     async fn test_tray_configuration_get_one() {
//         let db = setup_test_db().await;
//         let app = setup_test_app().await;
//         cleanup_test_data(&db).await;

//         // Create a tray configuration first
//         let create_input = json!({
//             "name": "Test Get Configuration",
//             "experiment_default": false,
//             "trays": [
//                 {
//                     "order_sequence": 1,
//                     "rotation_degrees": 0,
//                     "trays": [
//                         {
//                             "name": "Single test tray",
//                             "qty_x_axis": 8,
//                             "qty_y_axis": 6,
//                             "well_relative_diameter": "5.0"
//                         }
//                     ]
//                 }
//             ]
//         });

//         let create_response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("POST")
//                     .uri("/api/trays")
//                     .header("content-type", "application/json")
//                     .body(Body::from(create_input.to_string()))
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (create_status, create_body) = extract_response_body(create_response).await;
//         assert_eq!(create_status, StatusCode::CREATED);
//         let tray_config_id = create_body["id"].as_str().unwrap();

//         // Now get the tray configuration by ID
//         let get_response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("GET")
//                     .uri(&format!("/api/trays/{}", tray_config_id))
//                     .body(Body::empty())
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (get_status, get_body) = extract_response_body(get_response).await;
//         assert_eq!(
//             get_status,
//             StatusCode::OK,
//             "Failed to get tray configuration: {:?}",
//             get_body
//         );

//         // Validate the retrieved data matches what we created
//         assert_eq!(get_body["id"], tray_config_id);
//         assert_eq!(get_body["name"], "Test Get Configuration");
//         assert_eq!(get_body["experiment_default"], false);
//         assert_eq!(get_body["trays"].as_array().unwrap().len(), 1);

//         cleanup_test_data(&db).await;
//     }

//     #[tokio::test]
//     async fn test_tray_configuration_update() {
//         let db = setup_test_db().await;
//         let app = setup_test_app().await;
//         cleanup_test_data(&db).await;

//         // Create initial configuration
//         let create_input = json!({
//             "name": "Original Configuration",
//             "experiment_default": false,
//             "trays": [
//                 {
//                     "order_sequence": 1,
//                     "rotation_degrees": 0,
//                     "trays": [
//                         {
//                             "name": "Original tray",
//                             "qty_x_axis": 8,
//                             "qty_y_axis": 6,
//                             "well_relative_diameter": "5.0"
//                         }
//                     ]
//                 }
//             ]
//         });

//         let create_response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("POST")
//                     .uri("/api/trays")
//                     .header("content-type", "application/json")
//                     .body(Body::from(create_input.to_string()))
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (create_status, create_body) = extract_response_body(create_response).await;
//         assert_eq!(create_status, StatusCode::CREATED);
//         let tray_config_id = create_body["id"].as_str().unwrap();

//         // Update the configuration
//         let update_input = json!({
//             "name": "Updated Configuration",
//             "experiment_default": true,
//             "trays": [
//                 {
//                     "order_sequence": 1,
//                     "rotation_degrees": 180,
//                     "trays": [
//                         {
//                             "name": "Updated tray",
//                             "qty_x_axis": 12,
//                             "qty_y_axis": 8,
//                             "well_relative_diameter": "6.5"
//                         }
//                     ]
//                 },
//                 {
//                     "order_sequence": 2,
//                     "rotation_degrees": 0,
//                     "trays": [
//                         {
//                             "name": "Additional tray",
//                             "qty_x_axis": 6,
//                             "qty_y_axis": 4,
//                             "well_relative_diameter": "4.0"
//                         }
//                     ]
//                 }
//             ]
//         });

//         let update_response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("PATCH")
//                     .uri(&format!("/api/trays/{}", tray_config_id))
//                     .header("content-type", "application/json")
//                     .body(Body::from(update_input.to_string()))
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (update_status, update_body) = extract_response_body(update_response).await;
//         assert_eq!(
//             update_status,
//             StatusCode::OK,
//             "Failed to update tray configuration: {:?}",
//             update_body
//         );

//         // Validate the updated data
//         assert_eq!(update_body["name"], "Updated Configuration");
//         assert_eq!(update_body["experiment_default"], true);
//         assert_eq!(update_body["trays"].as_array().unwrap().len(), 2);

//         // Check first tray assignment was updated
//         let first_tray = &update_body["trays"][0];
//         assert_eq!(first_tray["rotation_degrees"], 180);
//         assert_eq!(first_tray["trays"][0]["name"], "Updated tray");

//         cleanup_test_data(&db).await;
//     }

//     #[tokio::test]
//     async fn test_tray_configuration_list() {
//         let db = setup_test_db().await;
//         let app = setup_test_app().await;
//         cleanup_test_data(&db).await;

//         // Create multiple tray configurations
//         let configs = vec![
//             json!({
//                 "name": "Config A",
//                 "experiment_default": false,
//                 "trays": []
//             }),
//             json!({
//                 "name": "Config B",
//                 "experiment_default": true,
//                 "trays": []
//             }),
//             json!({
//                 "name": "Config C",
//                 "experiment_default": false,
//                 "trays": []
//             }),
//         ];

//         for config in configs {
//             let response = app
//                 .clone()
//                 .oneshot(
//                     Request::builder()
//                         .method("POST")
//                         .uri("/api/trays")
//                         .header("content-type", "application/json")
//                         .body(Body::from(config.to_string()))
//                         .unwrap(),
//                 )
//                 .await
//                 .unwrap();

//             let (status, _) = extract_response_body(response).await;
//             assert_eq!(status, StatusCode::CREATED);
//         }

//         // Get all configurations
//         let list_response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("GET")
//                     .uri("/api/trays")
//                     .body(Body::empty())
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (list_status, list_body) = extract_response_body(list_response).await;
//         assert_eq!(list_status, StatusCode::OK);

//         let items = list_body["items"].as_array().unwrap();
//         assert_eq!(items.len(), 3, "Should return all 3 created configurations");

//         // Verify that only Config B has experiment_default = true
//         let default_configs: Vec<_> = items
//             .iter()
//             .filter(|item| item["experiment_default"] == true)
//             .collect();
//         assert_eq!(
//             default_configs.len(),
//             1,
//             "Only one configuration should be experiment_default"
//         );
//         assert_eq!(default_configs[0]["name"], "Config B");

//         cleanup_test_data(&db).await;
//     }

//     #[tokio::test]
//     async fn test_tray_configuration_delete() {
//         let db = setup_test_db().await;
//         let app = setup_test_app().await;
//         cleanup_test_data(&db).await;

//         // Create a configuration to delete
//         let create_input = json!({
//             "name": "To Be Deleted",
//             "experiment_default": false,
//             "trays": [
//                 {
//                     "order_sequence": 1,
//                     "rotation_degrees": 0,
//                     "trays": [
//                         {
//                             "name": "Doomed tray",
//                             "qty_x_axis": 8,
//                             "qty_y_axis": 6,
//                             "well_relative_diameter": "5.0"
//                         }
//                     ]
//                 }
//             ]
//         });

//         let create_response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("POST")
//                     .uri("/api/trays")
//                     .header("content-type", "application/json")
//                     .body(Body::from(create_input.to_string()))
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (create_status, create_body) = extract_response_body(create_response).await;
//         assert_eq!(create_status, StatusCode::CREATED);
//         let tray_config_id = create_body["id"].as_str().unwrap();

//         // Delete the configuration
//         let delete_response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("DELETE")
//                     .uri(&format!("/api/trays/{}", tray_config_id))
//                     .body(Body::empty())
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (delete_status, _) = extract_response_body(delete_response).await;
//         assert_eq!(delete_status, StatusCode::NO_CONTENT);

//         // Verify it's actually deleted
//         let get_response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("GET")
//                     .uri(&format!("/api/trays/{}", tray_config_id))
//                     .body(Body::empty())
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (get_status, _) = extract_response_body(get_response).await;
//         assert_eq!(get_status, StatusCode::NOT_FOUND);

//         cleanup_test_data(&db).await;
//     }

//     #[tokio::test]
//     async fn test_tray_configuration_validation_errors() {
//         let db = setup_test_db().await;
//         let app = setup_test_app().await;
//         cleanup_test_data(&db).await;

//         // Test with null name
//         let invalid_data = json!({
//             "name": null,
//             "experiment_default": false,
//             "trays": []
//         });

//         let response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("POST")
//                     .uri("/api/trays")
//                     .header("content-type", "application/json")
//                     .body(Body::from(invalid_data.to_string()))
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (status, body) = extract_response_body(response).await;
//         assert!(
//             status.is_client_error(),
//             "Should reject tray configuration with null name. Status: {status:?}, Body: {body:?}"
//         );

//         // Test with missing required fields
//         let incomplete_data = json!({
//             "name": "Incomplete Config"
//             // Missing experiment_default
//         });

//         let response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("POST")
//                     .uri("/api/trays")
//                     .header("content-type", "application/json")
//                     .body(Body::from(incomplete_data.to_string()))
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (status, body) = extract_response_body(response).await;
//         assert!(
//             status.is_client_error(),
//             "Should reject incomplete configuration. Status: {status:?}, Body: {body:?}"
//         );

//         cleanup_test_data(&db).await;
//     }

//     #[tokio::test]
//     async fn test_experiment_default_exclusivity() {
//         let db = setup_test_db().await;
//         let app = setup_test_app().await;
//         cleanup_test_data(&db).await;

//         // Create first configuration as experiment_default
//         let first_config = json!({
//             "name": "First Default",
//             "experiment_default": true,
//             "trays": []
//         });

//         let response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("POST")
//                     .uri("/api/trays")
//                     .header("content-type", "application/json")
//                     .body(Body::from(first_config.to_string()))
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (status, body) = extract_response_body(response).await;
//         assert_eq!(status, StatusCode::CREATED);
//         let first_id = body["id"].as_str().unwrap();

//         // Create second configuration as experiment_default
//         let second_config = json!({
//             "name": "Second Default",
//             "experiment_default": true,
//             "trays": []
//         });

//         let response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("POST")
//                     .uri("/api/trays")
//                     .header("content-type", "application/json")
//                     .body(Body::from(second_config.to_string()))
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (status, _) = extract_response_body(response).await;
//         assert_eq!(status, StatusCode::CREATED);

//         // Verify first configuration is no longer experiment_default
//         let get_first_response = app
//             .clone()
//             .oneshot(
//                 Request::builder()
//                     .method("GET")
//                     .uri(&format!("/api/trays/{}", first_id))
//                     .body(Body::empty())
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         let (get_status, get_body) = extract_response_body(get_first_response).await;
//         assert_eq!(get_status, StatusCode::OK);
//         assert_eq!(
//             get_body["experiment_default"], false,
//             "First configuration should no longer be experiment_default"
//         );

//         cleanup_test_data(&db).await;
//     }
// }
