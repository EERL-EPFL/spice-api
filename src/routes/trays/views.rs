use super::models::{TrayConfiguration, TrayConfigurationCreate, TrayConfigurationUpdate};
use crate::common::auth::Role;
use crate::common::state::AppState;
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::{CRUDResource, crud_handlers};
use utoipa_axum::{router::OpenApiRouter, routes};

// Generate CRUD handlers for TrayConfiguration (this will be for /api/tray-configurations endpoint)
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

#[cfg(test)]
mod tests {
    use crate::config::test_helpers::setup_test_app;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::{Value, json};
    use tower::ServiceExt;

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

    async fn create_test_tray_via_api(
        app: &axum::Router,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let tray_data = json!({
            "name": "TestTray",
            "experiment_default": true,
            "trays": [
                {
                    "trays": [
                        {
                            "name": "P1",
                            "qty_x_axis": 8,
                            "qty_y_axis": 12,
                            "well_relative_diameter": 0.6
                        }
                    ],
                    "rotation_degrees": 0,
                    "order_sequence": 1
                },
                {
                    "trays": [
                        {
                            "name": "P2",
                            "qty_x_axis": 8,
                            "qty_y_axis": 12,
                            "well_relative_diameter": 0.6
                        }
                    ],
                    "rotation_degrees": 180,
                    "order_sequence": 2
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
                    .body(Body::from(tray_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body) = extract_response_body(response).await;
        if status == StatusCode::CREATED {
            Ok(body)
        } else {
            Err(format!("Failed to create tray: {status:?}, body: {body:?}").into())
        }
    }

    #[tokio::test]
    async fn test_create_tray() {
        let app = setup_test_app().await;

        let tray_data = json!({
            "name": "TestTray",
            "experiment_default": true,
            "trays": [
                {
                    "trays": [
                        {
                            "name": "P1",
                            "qty_x_axis": 8,
                            "qty_y_axis": 12,
                            "well_relative_diameter": 0.6
                        }
                    ],
                    "rotation_degrees": 0,
                    "order_sequence": 1
                },
                {
                    "trays": [
                        {
                            "name": "P2",
                            "qty_x_axis": 8,
                            "qty_y_axis": 12,
                            "well_relative_diameter": 0.6
                        }
                    ],
                    "rotation_degrees": 180,
                    "order_sequence": 2
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

        // Validate response structure matches expected format
        assert!(body["id"].is_string());
        assert_eq!(body["name"], "TestTray");
        assert_eq!(body["experiment_default"], true);
        assert!(body["created_at"].is_string());
        assert!(body["last_updated"].is_string());
        assert!(body["trays"].is_array());
        assert_eq!(body["trays"].as_array().unwrap().len(), 2);
        assert!(body["associated_experiments"].is_array());

        // Validate tray structure
        let first_tray = &body["trays"][0];
        assert_eq!(first_tray["order_sequence"], 1);
        assert_eq!(first_tray["rotation_degrees"], 0);
        assert_eq!(first_tray["trays"][0]["name"], "P1");

        let second_tray = &body["trays"][1];
        assert_eq!(second_tray["order_sequence"], 2);
        assert_eq!(second_tray["rotation_degrees"], 180);
        assert_eq!(second_tray["trays"][0]["name"], "P2");
    }

    #[tokio::test]
    async fn test_get_tray_by_id() {
        let app = setup_test_app().await;

        // Create a tray first
        let tray = create_test_tray_via_api(&app)
            .await
            .expect("Failed to create test tray");
        let tray_id = tray["id"].as_str().unwrap();

        // Get the tray by ID
        let response = app
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

        let (status, body) = extract_response_body(response).await;
        assert_eq!(status, StatusCode::OK, "Failed to get tray: {body:?}");
        assert_eq!(body["id"], tray_id);
        assert_eq!(body["name"], "TestTray");
        assert_eq!(body["experiment_default"], true);
    }

    #[tokio::test]
    async fn test_list_trays() {
        let app = setup_test_app().await;

        // Create a few trays
        for i in 1..=3 {
            let tray_data = json!({
                "name": format!("TestTray{}", i),
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
                        .body(Body::from(tray_data.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();

            let (status, _) = extract_response_body(response).await;
            assert_eq!(status, StatusCode::CREATED);
        }

        // List all trays
        let response = app
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

        let (status, body) = extract_response_body(response).await;
        assert_eq!(status, StatusCode::OK, "Failed to list trays: {body:?}");
        assert!(body.is_array(), "Response should be an array");
        let items = body.as_array().unwrap();
        assert!(items.len() >= 3, "Should have at least 3 trays");
    }

    #[tokio::test]
    async fn test_update_tray() {
        let app = setup_test_app().await;

        // Create a tray first
        let tray = create_test_tray_via_api(&app)
            .await
            .expect("Failed to create test tray");
        let tray_id = tray["id"].as_str().unwrap();

        // Update the tray - use same format as create
        let update_data = json!({
            "name": "UpdatedTestTray",
            "experiment_default": false,
            "trays": [
                {
                    "trays": [
                        {
                            "name": "UpdatedP1",
                            "qty_x_axis": 10,
                            "qty_y_axis": 14,
                            "well_relative_diameter": 0.8
                        }
                    ],
                    "rotation_degrees": 90,
                    "order_sequence": 1
                }
            ]
        });

        let response = app
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

        let (status, body) = extract_response_body(response).await;

        // If PUT doesn't work, try PATCH
        if status == StatusCode::OK {
            assert_eq!(body["name"], "UpdatedTestTray");
            assert_eq!(body["experiment_default"], false);
            assert_eq!(body["trays"].as_array().unwrap().len(), 1);
        } else {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("PATCH")
                        .uri(format!("/api/trays/{tray_id}"))
                        .header("content-type", "application/json")
                        .body(Body::from(update_data.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();

            let (status, body) = extract_response_body(response).await;
            assert_eq!(status, StatusCode::OK, "Failed to update tray: {body:?}");
            assert_eq!(body["name"], "UpdatedTestTray");
            assert_eq!(body["experiment_default"], false);
            assert_eq!(body["trays"].as_array().unwrap().len(), 1);
        }
    }

    #[tokio::test]
    async fn test_delete_tray() {
        let app = setup_test_app().await;

        // Create a tray first
        let tray = create_test_tray_via_api(&app)
            .await
            .expect("Failed to create test tray");
        let tray_id = tray["id"].as_str().unwrap();

        // Delete the tray
        let response = app
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

        let (status, _) = extract_response_body(response).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "Failed to delete tray");

        // Verify it's actually deleted
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

        let (get_status, _) = extract_response_body(get_response).await;
        assert_eq!(get_status, StatusCode::NOT_FOUND, "Tray should be deleted");
    }

    #[tokio::test]
    async fn test_experiment_default_exclusivity() {
        let app = setup_test_app().await;

        // Create first tray as experiment_default
        let first_tray_data = json!({
            "name": "FirstDefault",
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
                    .body(Body::from(first_tray_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body) = extract_response_body(response).await;
        assert_eq!(status, StatusCode::CREATED);
        let first_tray_id = body["id"].as_str().unwrap();

        // Create second tray as experiment_default
        let second_tray_data = json!({
            "name": "SecondDefault",
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
                    .body(Body::from(second_tray_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, _) = extract_response_body(response).await;
        assert_eq!(status, StatusCode::CREATED);

        // Verify first tray is no longer experiment_default
        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/trays/{first_tray_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (get_status, get_body) = extract_response_body(get_response).await;
        assert_eq!(get_status, StatusCode::OK);
        assert_eq!(
            get_body["experiment_default"], false,
            "First tray should no longer be experiment_default"
        );
    }

    #[tokio::test]
    async fn test_validation_errors() {
        let app = setup_test_app().await;

        // Test with missing required fields
        let invalid_data = json!({
            "name": "Invalid Tray"
            // Missing experiment_default
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

        let (status, _) = extract_response_body(response).await;
        assert!(
            status.is_client_error(),
            "Should reject tray with missing required fields"
        );
    }
}
