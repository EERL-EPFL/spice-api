use super::models::{Tray, TrayCreate, TrayUpdate};
use crate::common::auth::Role;
use crate::common::state::AppState;
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::{CRUDResource, crud_handlers};
use utoipa_axum::{router::OpenApiRouter, routes};

// Generate CRUD handlers for Tray (this will be for /api/trays endpoint)
crud_handlers!(Tray, TrayUpdate, TrayCreate);

pub fn router(state: &AppState) -> OpenApiRouter
where
    Tray: CRUDResource,
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
            Tray::RESOURCE_NAME_PLURAL
        );
    }

    mutating_router
}

#[cfg(test)]
mod tests {
    use crate::config::test_helpers::{cleanup_test_data, setup_test_app, setup_test_db};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use sea_orm::{DatabaseConnection, EntityTrait, Set};
    use serde_json::{Value, json};
    use spice_entity::prelude::*;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn extract_response_body(response: axum::response::Response) -> (StatusCode, Value) {
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let body_json: Value = if body_str.is_empty() {
            json!(null)
        } else {
            serde_json::from_str(&body_str).unwrap_or(json!(body_str))
        };
        (status, body_json)
    }

    async fn create_test_tray(
        db: &DatabaseConnection,
    ) -> Result<spice_entity::trays::Model, Box<dyn std::error::Error>> {
        let tray = spice_entity::trays::ActiveModel {
            id: Set(Uuid::new_v4()),
            name: Set(Some("TEST_TRAY".to_string())),
            ..Default::default()
        };

        let tray = Trays::insert(tray).exec_with_returning(db).await?;
        Ok(tray)
    }

    async fn create_test_tray_configuration(
        db: &DatabaseConnection,
    ) -> Result<spice_entity::tray_configurations::Model, Box<dyn std::error::Error>> {
        let config = spice_entity::tray_configurations::ActiveModel {
            id: Set(Uuid::new_v4()),
            name: Set(Some("TEST_CONFIG".to_string())),
            experiment_default: Set(false), // Add the missing required field
            created_at: Set(chrono::Utc::now().into()),
            last_updated: Set(chrono::Utc::now().into()),
        };

        let config = spice_entity::tray_configurations::Entity::insert(config)
            .exec_with_returning(db)
            .await?;
        Ok(config)
    }

    #[tokio::test]
    async fn test_create_tray_configuration_assignment() {
        let app = setup_test_app().await;
        let db = setup_test_db().await;

        let tray = create_test_tray(&db)
            .await
            .expect("Failed to create test tray");
        let config = create_test_tray_configuration(&db)
            .await
            .expect("Failed to create test config");

        let request_body = json!({
            "tray_id": tray.id,
            "tray_configuration_id": config.id
        });

        let request = Request::builder()
            .method("POST")
            .uri("/api/trays/configuration-assignments")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let (status, _body) = extract_response_body(response).await;

        assert_eq!(status, StatusCode::CREATED);
        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_get_tray_wells() {
        let app = setup_test_app().await;
        let db = setup_test_db().await;

        let tray = create_test_tray(&db)
            .await
            .expect("Failed to create test tray");

        let request = Request::builder()
            .method("GET")
            .uri(format!("/api/trays/{}/wells", tray.id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let (status, body) = extract_response_body(response).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.is_array());
        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_get_tray_by_id() {
        let app = setup_test_app().await;
        let db = setup_test_db().await;

        let tray = create_test_tray(&db)
            .await
            .expect("Failed to create test tray");

        let request = Request::builder()
            .method("GET")
            .uri(format!("/api/trays/{}", tray.id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let (status, body) = extract_response_body(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], tray.id.to_string());
        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_list_trays() {
        let app = setup_test_app().await;
        let db = setup_test_db().await;

        let request = Request::builder()
            .method("GET")
            .uri("/api/trays")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let (status, body) = extract_response_body(response).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.is_array());
        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_list_tray_configurations() {
        let app = setup_test_app().await;
        let db = setup_test_db().await;

        let request = Request::builder()
            .method("GET")
            .uri("/api/tray-configurations")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let (status, body) = extract_response_body(response).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.is_array());
        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_tray_configuration_create_complete() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Test creating a comprehensive tray configuration
        let tray_config_input = json!({
            "name": "Standard 96-well Configuration",
            "experiment_default": true,
            "trays": [
                {
                    "order_sequence": 1,
                    "rotation_degrees": 0,
                    "trays": [
                        {
                            "name": "96-well plate A",
                            "qty_x_axis": 12,
                            "qty_y_axis": 8,
                            "well_relative_diameter": "6.35"
                        }
                    ]
                },
                {
                    "order_sequence": 2,
                    "rotation_degrees": 90,
                    "trays": [
                        {
                            "name": "96-well plate B",
                            "qty_x_axis": 12,
                            "qty_y_axis": 8,
                            "well_relative_diameter": "6.35"
                        }
                    ]
                }
            ]
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trays")
                    .header("content-type", "application/json")
                    .body(Body::from(tray_config_input.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body) = extract_response_body(response).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Failed to create tray configuration: {body:?}"
        );

        // Validate response structure
        assert!(body["id"].is_string(), "Response should include ID");
        assert_eq!(body["name"], "Standard 96-well Configuration");
        assert_eq!(body["experiment_default"], true);
        assert!(body["created_at"].is_string());
        assert!(body["last_updated"].is_string());
        assert!(body["trays"].is_array());
        assert_eq!(body["trays"].as_array().unwrap().len(), 2);

        // Validate tray assignments structure
        let first_tray = &body["trays"][0];
        assert_eq!(first_tray["order_sequence"], 1);
        assert_eq!(first_tray["rotation_degrees"], 0);
        assert!(first_tray["trays"].is_array());

        let nested_tray = &first_tray["trays"][0];
        assert_eq!(nested_tray["name"], "96-well plate A");
        assert_eq!(nested_tray["qty_x_axis"], 12);
        assert_eq!(nested_tray["qty_y_axis"], 8);

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_tray_configuration_get_one() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Create a tray configuration first
        let create_input = json!({
            "name": "Test Get Configuration",
            "experiment_default": false,
            "trays": [
                {
                    "order_sequence": 1,
                    "rotation_degrees": 0,
                    "trays": [
                        {
                            "name": "Single test tray",
                            "qty_x_axis": 8,
                            "qty_y_axis": 6,
                            "well_relative_diameter": "5.0"
                        }
                    ]
                }
            ]
        });

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trays")
                    .header("content-type", "application/json")
                    .body(Body::from(create_input.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (create_status, create_body) = extract_response_body(create_response).await;
        assert_eq!(create_status, StatusCode::CREATED);
        let tray_config_id = create_body["id"].as_str().unwrap();

        // Now get the tray configuration by ID
        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/trays/{tray_config_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (get_status, get_body) = extract_response_body(get_response).await;
        assert_eq!(
            get_status,
            StatusCode::OK,
            "Failed to get tray configuration: {get_body:?}"
        );

        // Validate the retrieved data matches what we created
        assert_eq!(get_body["id"], tray_config_id);
        assert_eq!(get_body["name"], "Test Get Configuration");
        assert_eq!(get_body["experiment_default"], false);
        assert_eq!(get_body["trays"].as_array().unwrap().len(), 1);

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_tray_configuration_update() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Create initial configuration
        let create_input = json!({
            "name": "Original Configuration",
            "experiment_default": false,
            "trays": [
                {
                    "order_sequence": 1,
                    "rotation_degrees": 0,
                    "trays": [
                        {
                            "name": "Original tray",
                            "qty_x_axis": 8,
                            "qty_y_axis": 6,
                            "well_relative_diameter": "5.0"
                        }
                    ]
                }
            ]
        });

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trays")
                    .header("content-type", "application/json")
                    .body(Body::from(create_input.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (create_status, create_body) = extract_response_body(create_response).await;
        assert_eq!(create_status, StatusCode::CREATED);
        let tray_config_id = create_body["id"].as_str().unwrap();

        // Update the configuration
        let update_input = json!({
            "name": "Updated Configuration",
            "experiment_default": true,
            "trays": [
                {
                    "order_sequence": 1,
                    "rotation_degrees": 180,
                    "trays": [
                        {
                            "name": "Updated tray",
                            "qty_x_axis": 12,
                            "qty_y_axis": 8,
                            "well_relative_diameter": "6.5"
                        }
                    ]
                },
                {
                    "order_sequence": 2,
                    "rotation_degrees": 0,
                    "trays": [
                        {
                            "name": "Additional tray",
                            "qty_x_axis": 6,
                            "qty_y_axis": 4,
                            "well_relative_diameter": "4.0"
                        }
                    ]
                }
            ]
        });

        let update_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/trays/{tray_config_id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(update_input.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (update_status, update_body) = extract_response_body(update_response).await;
        assert_eq!(
            update_status,
            StatusCode::OK,
            "Failed to update tray configuration: {update_body:?}"
        );

        // Validate the updated data
        assert_eq!(update_body["name"], "Updated Configuration");
        assert_eq!(update_body["experiment_default"], true);
        assert_eq!(update_body["trays"].as_array().unwrap().len(), 2);

        // Check first tray assignment was updated
        let first_tray = &update_body["trays"][0];
        assert_eq!(first_tray["rotation_degrees"], 180);
        assert_eq!(first_tray["trays"][0]["name"], "Updated tray");

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_tray_configuration_list() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Create multiple tray configurations
        let configs = vec![
            json!({
                "name": "Config A",
                "experiment_default": false,
                "trays": []
            }),
            json!({
                "name": "Config B",
                "experiment_default": true,
                "trays": []
            }),
            json!({
                "name": "Config C",
                "experiment_default": false,
                "trays": []
            }),
        ];

        for config in configs {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/trays")
                        .header("content-type", "application/json")
                        .body(Body::from(config.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();

            let (status, _) = extract_response_body(response).await;
            assert_eq!(status, StatusCode::CREATED);
        }

        // Get all configurations
        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/trays")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (list_status, list_body) = extract_response_body(list_response).await;
        assert_eq!(list_status, StatusCode::OK);

        let items = list_body["items"].as_array().unwrap();
        assert_eq!(items.len(), 3, "Should return all 3 created configurations");

        // Verify that only Config B has experiment_default = true
        let default_configs: Vec<_> = items
            .iter()
            .filter(|item| item["experiment_default"] == true)
            .collect();
        assert_eq!(
            default_configs.len(),
            1,
            "Only one configuration should be experiment_default"
        );
        assert_eq!(default_configs[0]["name"], "Config B");

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_tray_configuration_delete() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Create a configuration to delete
        let create_input = json!({
            "name": "To Be Deleted",
            "experiment_default": false,
            "trays": [
                {
                    "order_sequence": 1,
                    "rotation_degrees": 0,
                    "trays": [
                        {
                            "name": "Doomed tray",
                            "qty_x_axis": 8,
                            "qty_y_axis": 6,
                            "well_relative_diameter": "5.0"
                        }
                    ]
                }
            ]
        });

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trays")
                    .header("content-type", "application/json")
                    .body(Body::from(create_input.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (create_status, create_body) = extract_response_body(create_response).await;
        assert_eq!(create_status, StatusCode::CREATED);
        let tray_config_id = create_body["id"].as_str().unwrap();

        // Delete the configuration
        let delete_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/trays/{tray_config_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (delete_status, _) = extract_response_body(delete_response).await;
        assert_eq!(delete_status, StatusCode::NO_CONTENT);

        // Verify it's actually deleted
        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/trays/{tray_config_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (get_status, _) = extract_response_body(get_response).await;
        assert_eq!(get_status, StatusCode::NOT_FOUND);

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_tray_configuration_validation_errors() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Test with null name
        let invalid_data = json!({
            "name": null,
            "experiment_default": false,
            "trays": []
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trays")
                    .header("content-type", "application/json")
                    .body(Body::from(invalid_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body) = extract_response_body(response).await;
        assert!(
            status.is_client_error(),
            "Should reject tray configuration with null name. Status: {status:?}, Body: {body:?}"
        );

        // Test with missing required fields
        let incomplete_data = json!({
            "name": "Incomplete Config"
            // Missing experiment_default
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trays")
                    .header("content-type", "application/json")
                    .body(Body::from(incomplete_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body) = extract_response_body(response).await;
        assert!(
            status.is_client_error(),
            "Should reject incomplete configuration. Status: {status:?}, Body: {body:?}"
        );

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_experiment_default_exclusivity() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Create first configuration as experiment_default
        let first_config = json!({
            "name": "First Default",
            "experiment_default": true,
            "trays": []
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trays")
                    .header("content-type", "application/json")
                    .body(Body::from(first_config.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body) = extract_response_body(response).await;
        assert_eq!(status, StatusCode::CREATED);
        let first_id = body["id"].as_str().unwrap();

        // Create second configuration as experiment_default
        let second_config = json!({
            "name": "Second Default",
            "experiment_default": true,
            "trays": []
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trays")
                    .header("content-type", "application/json")
                    .body(Body::from(second_config.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, _) = extract_response_body(response).await;
        assert_eq!(status, StatusCode::CREATED);

        // Verify first configuration is no longer experiment_default
        let get_first_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/trays/{first_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (get_status, get_body) = extract_response_body(get_first_response).await;
        assert_eq!(get_status, StatusCode::OK);
        assert_eq!(
            get_body["experiment_default"], false,
            "First configuration should no longer be experiment_default"
        );

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_tray_crud_operations() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Test creating a tray
        let tray_data = json!({
            "name": format!("Test Tray {}", uuid::Uuid::new_v4()),
            "qty_x_axis": 8,
            "qty_y_axis": 12,
            "well_relative_diameter": 0.85
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trays")
                    .header("content-type", "application/json")
                    .body(Body::from(tray_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body) = extract_response_body(response).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Failed to create tray: {body:?}"
        );

        let tray_id = body["id"].as_str().unwrap();

        // Test reading the created tray
        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/trays/{tray_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (get_status, get_body) = extract_response_body(get_response).await;
        assert_eq!(get_status, StatusCode::OK, "Failed to get tray");
        assert_eq!(get_body["id"], tray_id);
        assert_eq!(get_body["qty_x_axis"], 8);
        assert_eq!(get_body["qty_y_axis"], 12);

        // Test updating the tray
        let update_data = json!({
            "name": format!("Updated Tray {}", uuid::Uuid::new_v4()),
            "qty_x_axis": 10,
            "qty_y_axis": 14,
            "well_relative_diameter": 0.9
        });

        let update_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/trays/{tray_id}"))
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
            "Failed to update tray: {update_body:?}"
        );
        assert_eq!(update_body["qty_x_axis"], 10);
        assert_eq!(update_body["qty_y_axis"], 14);

        // Test deleting the tray
        let delete_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/trays/{tray_id}"))
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
    async fn test_tray_list_operations() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Create multiple trays for listing tests
        for i in 1..=3 {
            let tray_data = json!({
                "name": format!("List Test Tray {}", i),
                "qty_x_axis": 8,
                "qty_y_axis": 12
            });

            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/trays")
                        .header("content-type", "application/json")
                        .body(Body::from(tray_data.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();

            let (status, _) = extract_response_body(response).await;
            assert_eq!(status, StatusCode::CREATED);
        }

        // Test listing all trays
        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/trays")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (list_status, list_body) = extract_response_body(list_response).await;
        assert_eq!(list_status, StatusCode::OK, "Failed to list trays");
        assert!(list_body["items"].is_array());
        let items = list_body["items"].as_array().unwrap();
        assert!(items.len() >= 3, "Should find at least 3 trays");

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_tray_validation() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Test creating tray with invalid data (negative dimensions)
        let invalid_data = json!({
            "name": "Invalid Tray",
            "qty_x_axis": -1,
            "qty_y_axis": 12
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trays")
                    .header("content-type", "application/json")
                    .body(Body::from(invalid_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, _body) = extract_response_body(response).await;
        assert!(
            status.is_client_error(),
            "Should reject tray with negative dimensions"
        );

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_tray_configuration_crud() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Test creating a tray configuration
        let config_data = json!({
            "name": format!("Test Config {}", uuid::Uuid::new_v4()),
            "experiment_default": true
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tray_configurations")
                    .header("content-type", "application/json")
                    .body(Body::from(config_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body) = extract_response_body(response).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Failed to create tray configuration: {body:?}"
        );

        let config_id = body["id"].as_str().unwrap();

        // Test reading the created configuration
        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/tray_configurations/{config_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (get_status, get_body) = extract_response_body(get_response).await;
        assert_eq!(
            get_status,
            StatusCode::OK,
            "Failed to get tray configuration"
        );
        assert_eq!(get_body["id"], config_id);
        assert_eq!(get_body["experiment_default"], true);

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_tray_configuration_assignments() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Create a tray and configuration first
        let tray = create_test_tray(&db)
            .await
            .expect("Failed to create test tray");
        let config = create_test_tray_configuration(&db)
            .await
            .expect("Failed to create test configuration");

        // Test creating a tray configuration assignment
        let assignment_data = json!({
            "tray_id": tray.id,
            "tray_configuration_id": config.id,
            "order_sequence": 1,
            "rotation_degrees": 0
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tray_configuration_assignments")
                    .header("content-type", "application/json")
                    .body(Body::from(assignment_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body) = extract_response_body(response).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Failed to create tray configuration assignment: {body:?}"
        );

        // Test listing assignments for the configuration
        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/tray_configuration_assignments?filter[tray_configuration_id]={}",
                        config.id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (list_status, list_body) = extract_response_body(list_response).await;
        assert_eq!(
            list_status,
            StatusCode::OK,
            "Failed to list tray configuration assignments"
        );
        let items = list_body["items"].as_array().unwrap();
        assert!(!items.is_empty(), "Should find at least 1 assignment");

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_tray_wells_relationship() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Create a tray first
        let tray = create_test_tray(&db)
            .await
            .expect("Failed to create test tray");

        // Test creating wells for the tray
        for row in 1..=3 {
            for col in 1..=3 {
                let well_data = json!({
                    "tray_id": tray.id,
                    "row_number": row,
                    "column_number": col
                });

                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("POST")
                            .uri("/api/wells")
                            .header("content-type", "application/json")
                            .body(Body::from(well_data.to_string()))
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                let (status, _) = extract_response_body(response).await;
                assert_eq!(
                    status,
                    StatusCode::CREATED,
                    "Failed to create well at ({row}, {col})"
                );
            }
        }

        // Test filtering wells by tray_id
        let wells_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/wells?filter[tray_id]={}", tray.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (wells_status, wells_body) = extract_response_body(wells_response).await;
        assert_eq!(wells_status, StatusCode::OK, "Failed to get wells for tray");
        let wells = wells_body["items"].as_array().unwrap();
        assert_eq!(wells.len(), 9, "Should have 9 wells (3x3 grid)");

        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_tray_pagination() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Create multiple trays for pagination testing
        for i in 1..=5 {
            let tray_data = json!({
                "name": format!("Pagination Test Tray {}", i),
                "qty_x_axis": 8,
                "qty_y_axis": 12
            });

            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/trays")
                        .header("content-type", "application/json")
                        .body(Body::from(tray_data.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();

            let (status, _) = extract_response_body(response).await;
            assert_eq!(status, StatusCode::CREATED);
        }

        // Test pagination with page_size=2
        let page_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/trays?page_size=2&page=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (page_status, page_body) = extract_response_body(page_response).await;
        assert_eq!(page_status, StatusCode::OK, "Failed to paginate trays");
        let paginated_items = page_body["items"].as_array().unwrap();
        assert!(
            paginated_items.len() <= 2,
            "Pagination should limit results to 2"
        );

        cleanup_test_data(&db).await;
    }
}
