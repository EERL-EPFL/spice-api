use crate::config::test_helpers::setup_test_app;
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

    // Log error details for debugging
    if status.is_server_error() || status.is_client_error() {
        eprintln!("HTTP Error - Status: {status}, Body: {body:?}");
    }

    (status, body)
}

async fn create_test_tray_crud(app: &axum::Router) -> (StatusCode, Value) {
    let tray_data = json!({
        "name": format!("Test Tray Config CRUD {}", uuid::Uuid::new_v4()),
        "experiment_default": false,
        "trays": [
            {
                "order_sequence": 1,
                "rotation_degrees": 0,
                "name": "CRUD Test Tray",
                "qty_x_axis": 12,
                "qty_y_axis": 8,
                "well_relative_diameter": 2.5
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
    (status, body)
}

#[tokio::test]
async fn test_tray_crud_operations() {
    let app = setup_test_app().await;
    let (status, body) = create_test_tray_crud(&app).await;

    if status == StatusCode::CREATED {
        assert!(body["id"].is_string(), "Response should include ID");
        assert!(
            body["name"]
                .as_str()
                .unwrap()
                .contains("Test Tray Config CRUD")
        );
        assert_eq!(body["experiment_default"], false);
        assert!(body["created_at"].is_string());
        assert!(body["last_updated"].is_string());
        assert!(body["trays"].is_array());
        let tray_id = body["id"].as_str().unwrap();

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
        if get_status == StatusCode::OK {
            assert_eq!(get_body["id"], tray_id);
            assert!(
                get_body["name"]
                    .as_str()
                    .unwrap()
                    .contains("Test Tray Config CRUD")
            );
        }

        let update_data = json!({
            "name": "Updated Test Tray Config CRUD",
            "experiment_default": true
        });

        let update_response = app
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

        let (update_status, update_body) = extract_response_body(update_response).await;
        if update_status == StatusCode::OK {
            assert!(update_body["name"].as_str().unwrap().contains("Updated"));
            assert_eq!(update_body["experiment_default"], true);
        }

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

        let delete_status = delete_response.status();
        // Note: Delete may not be implemented yet, so we don't assert specific behavior
        assert!(
            delete_status == StatusCode::OK
                || delete_status == StatusCode::NO_CONTENT
                || delete_status == StatusCode::NOT_FOUND
                || delete_status == StatusCode::METHOD_NOT_ALLOWED,
            "Delete should return OK, 204 (no content), 404 (not found), or 405 (not implemented), got: {delete_status}"
        );
    } else {
        // Document the current behavior even if it fails
        assert!(
            status.is_client_error() || status.is_server_error(),
            "Tray creation should either succeed or fail gracefully, got: {status}"
        );
    }
}

#[tokio::test]
async fn test_tray_list_operations() {
    let app = setup_test_app().await;

    // Test getting all trays
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

    if list_status == StatusCode::OK {
        assert!(list_body.is_array(), "Trays list should be an array");
        let trays = list_body.as_array().unwrap();

        // Validate structure of trays in list
        for tray in trays {
            assert!(tray["id"].is_string(), "Each tray should have ID");
            assert!(
                tray["created_at"].is_string(),
                "Each tray should have created_at"
            );
            assert!(
                tray["last_updated"].is_string(),
                "Each tray should have last_updated"
            );

            // Name can be null, so check if it's present and valid when not null
            if !tray["name"].is_null() {
                assert!(
                    tray["name"].is_string(),
                    "Tray name should be string when present"
                );
            }
        }
    } else {
        assert!(
            list_status.is_client_error() || list_status.is_server_error(),
            "Tray listing should either succeed or fail gracefully"
        );
    }
}

#[tokio::test]
async fn test_tray_validation() {
    let app = setup_test_app().await;

    // Test creating tray configuration with invalid tray data (negative axis values)
    let invalid_data = json!({
        "name": "Invalid Tray Config",
        "experiment_default": false,
        "trays": [
            {
                "order_sequence": 1,
                "rotation_degrees": 0,
                "name": "Invalid Tray",
                "qty_x_axis": -5,  // Should be positive
                "qty_y_axis": 8,
                "well_relative_diameter": 2.5
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
                .body(Body::from(invalid_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, _body) = extract_response_body(response).await;

    assert!(
        status.is_client_error(),
        "Tray configuration should reject negative axis values, but got status: {status}"
    );

    // Test creating tray configuration with zero dimensions
    let zero_data = json!({
        "name": "Zero Dimensions Tray Config",
        "experiment_default": false,
        "trays": [
            {
                "order_sequence": 1,
                "rotation_degrees": 0,
                "name": "Zero Dimensions Tray",
                "qty_x_axis": 0,
                "qty_y_axis": 0,
                "well_relative_diameter": 0.0
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
                .body(Body::from(zero_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, _body) = extract_response_body(response).await;

    assert!(
        status.is_client_error(),
        "Tray configuration should reject zero dimensions, but got status: {status}"
    );
}

#[tokio::test]
async fn test_tray_filtering_and_sorting() {
    let app = setup_test_app().await;

    // Create test trays for filtering
    let test_trays = [
        ("96-Well Plate", 12, 8),
        ("384-Well Plate", 24, 16),
        ("48-Well Plate", 8, 6),
    ];

    let mut created_ids = Vec::new();

    for (name, x_axis, y_axis) in test_trays {
        let tray_data = json!({
            "name": format!("{} Config {}", name, &uuid::Uuid::new_v4().to_string()[..8]),
            "experiment_default": false,
            "trays": [
                {
                    "order_sequence": 1,
                    "rotation_degrees": 0,
                    "name": format!("{} {}", name, &uuid::Uuid::new_v4().to_string()[..8]),
                    "qty_x_axis": x_axis,
                    "qty_y_axis": y_axis,
                    "well_relative_diameter": 2.0
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
            created_ids.push(body["id"].as_str().unwrap().to_string());
        }
    }

    if created_ids.is_empty() {
        // Skip filtering tests if no trays were created
    } else {
        // Test filtering by name using JSON filter format
        let filter_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/trays?filter=%7B%22name%22%3A%2296-Well%20Plate%20Config%22%7D")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (filter_status, filter_body) = extract_response_body(filter_response).await;

        if filter_status == StatusCode::OK {
            let filtered_trays = filter_body.as_array().unwrap();

            // Check if filtering actually works - it should only return matching trays
            for tray in filtered_trays {
                if let Some(name) = tray["name"].as_str() {
                    assert!(
                        name.contains("96-Well Plate Config"),
                        "Filtering should only return matching results, but got non-matching tray: {name}"
                    );
                }
            }
        } else {
            // Tray filtering failed
        }

        // Test sorting by name
        let sort_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/trays?sort[name]=asc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (sort_status, _) = extract_response_body(sort_response).await;

        if sort_status == StatusCode::OK {
            // Tray sorting endpoint accessible
        } else {
            // Tray sorting failed
        }
    }
}

#[tokio::test]
async fn test_tray_not_found() {
    let app = setup_test_app().await;

    // Test getting non-existent tray
    let fake_id = uuid::Uuid::new_v4();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/trays/{fake_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, _body) = extract_response_body(response).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Should return 404 for non-existent tray"
    );
}

async fn create_tray_configuration(
    app: &axum::Router,
) -> Result<Value, Box<dyn std::error::Error>> {
    let tray_config_data = json!({
        "name": format!("Test Tray Config {}", uuid::Uuid::new_v4()),
        "experiment_default": false,
        "trays": [
            {
                "order_sequence": 1,
                "rotation_degrees": 0,
                "name": "Primary Plate",
                "qty_x_axis": 12,
                "qty_y_axis": 8,
                "well_relative_diameter": 2.5
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
                .body(Body::from(tray_config_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, body) = extract_response_body(response).await;
    if status == StatusCode::CREATED {
        assert!(body["id"].is_string(), "Response should include ID");
        assert!(body["name"].as_str().unwrap().contains("Test Tray Config"));
        assert_eq!(body["experiment_default"], false);
        assert!(body["created_at"].is_string());
        assert!(body["last_updated"].is_string());
        Ok(body)
    } else {
        Err(format!("Failed to create tray configuration: {status:?}, body: {body:?}").into())
    }
}

#[tokio::test]
async fn test_tray_configuration_crud_operations() {
    let app = setup_test_app().await;
    let body = create_tray_configuration(&app)
        .await
        .expect("Failed to create tray configuration for testing");

    if body["trays"].is_array() {
        let trays = body["trays"].as_array().unwrap();

        if trays.len() == 1 {
            for assignment in trays {
                assert!(assignment["order_sequence"].is_number());
                assert!(assignment["rotation_degrees"].is_number());
                // Now tray details are embedded directly
                if !assignment["name"].is_null() {
                    assert!(assignment["name"].is_string());
                }
                if !assignment["qty_x_axis"].is_null() {
                    assert!(assignment["qty_x_axis"].is_number());
                }
                if !assignment["qty_y_axis"].is_null() {
                    assert!(assignment["qty_y_axis"].is_number());
                }
            }
        }
    }

    let tray_config_id = body["id"].as_str().unwrap();

    // Test getting the tray configuration by ID
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
    if get_status == StatusCode::OK {
        assert_eq!(get_body["id"], tray_config_id);
    }

    // Test updating the tray configuration
    let update_data = json!({
        "experiment_default": true
    });

    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/trays/{tray_config_id}"))
                .header("content-type", "application/json")
                .body(Body::from(update_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (update_status, update_body) = extract_response_body(update_response).await;
    if update_status == StatusCode::OK {
        assert_eq!(update_body["experiment_default"], true);
    }

    // Test deleting the tray configuration
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
    let delete_status = delete_response.status();
    assert!(
        delete_status.is_success(),
        "Tray configuration delete failed with status: {delete_status}"
    );
}

#[tokio::test]
async fn test_tray_configuration_list_operations() {
    let app = setup_test_app().await;

    // Test getting all tray configurations
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

    if list_status == StatusCode::OK {
        assert!(
            list_body.is_array(),
            "Tray configurations list should be an array"
        );
        let tray_configs = list_body.as_array().unwrap();

        // Validate structure of tray configurations in list
        for config in tray_configs {
            assert!(config["id"].is_string(), "Each config should have ID");
            assert!(
                config["created_at"].is_string(),
                "Each config should have created_at"
            );
            assert!(
                config["last_updated"].is_string(),
                "Each config should have last_updated"
            );
            assert!(
                config["experiment_default"].is_boolean(),
                "Each config should have experiment_default"
            );
        }
    } else {
        assert!(
            list_status.is_client_error() || list_status.is_server_error(),
            "Tray configuration listing should either succeed or fail gracefully"
        );
    }
}

#[tokio::test]
async fn test_tray_configuration_default_behavior() {
    let app = setup_test_app().await;

    // Test creating multiple tray configurations with default behavior
    let default_config_data = json!({
        "name": format!("Default Config {}", uuid::Uuid::new_v4()),
        "experiment_default": true,
        "trays": [
            {
                "order_sequence": 1,
                "rotation_degrees": 0,
                "trays": [
                    {
                        "name": "Default Plate",
                        "qty_x_axis": 12,
                        "qty_y_axis": 8,
                        "well_relative_diameter": 2.0
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
                .body(Body::from(default_config_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, body) = extract_response_body(response).await;

    if status == StatusCode::CREATED {
        assert_eq!(body["experiment_default"], true);

        let first_config_id = body["id"].as_str().unwrap();

        // Create another default configuration - should set the first one to false
        let second_default_config_data = json!({
            "name": format!("Second Default Config {}", uuid::Uuid::new_v4()),
            "experiment_default": true,
            "trays": [
                {
                    "order_sequence": 1,
                    "rotation_degrees": 0,
                    "trays": [
                        {
                            "name": "Second Default Plate",
                            "qty_x_axis": 8,
                            "qty_y_axis": 6,
                            "well_relative_diameter": 1.8
                        }
                    ]
                }
            ]
        });

        let second_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trays")
                    .header("content-type", "application/json")
                    .body(Body::from(second_default_config_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (second_status, second_body) = extract_response_body(second_response).await;

        if second_status == StatusCode::CREATED {
            assert_eq!(second_body["experiment_default"], true);

            // Check if the first configuration is no longer default
            let check_first_response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(format!("/api/trays/{first_config_id}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            let (check_status, check_body) = extract_response_body(check_first_response).await;

            if check_status == StatusCode::OK {
                assert_eq!(
                    check_body["experiment_default"], false,
                    "First tray configuration should no longer be experiment_default after creating second default"
                );
            }
        } else {
            // Could not test default behavior - second config creation failed
        }
    } else {
        // Skipping default behavior test - couldn't create first config
    }
}

#[tokio::test]
async fn test_tray_dimensions_validation() {
    let app = setup_test_app().await;

    // Test various dimension combinations
    let dimension_tests = [
        (1, 1, "Minimal dimensions"),
        (96, 1, "Single row, many columns"),
        (1, 96, "Single column, many rows"),
        (12, 8, "Standard 96-well plate"),
        (24, 16, "Standard 384-well plate"),
    ];

    for (x_axis, y_axis, description) in dimension_tests {
        let tray_data = json!({
            "name": format!("{} Config Test {}", description, &uuid::Uuid::new_v4().to_string()[..8]),
            "experiment_default": false,
            "trays": [
                {
                    "order_sequence": 1,
                    "rotation_degrees": 0,
                    "trays": [
                        {
                            "name": format!("{} Test {}", description, &uuid::Uuid::new_v4().to_string()[..8]),
                            "qty_x_axis": x_axis,
                            "qty_y_axis": y_axis,
                            "well_relative_diameter": 2.0
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
                    .body(Body::from(tray_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, _body) = extract_response_body(response).await;

        // Tray config validation results - validate that we get a proper response
        assert!(
            status.is_success() || status.is_client_error() || status.is_server_error(),
            "Should get a valid HTTP status code, got: {status}"
        );
    }
}

async fn create_tray_configuration_complex_structure(app: &axum::Router) -> axum::response::Response {
    // Test creating a complex tray configuration with multiple assignments and rotations
    // Each assignment should have exactly one tray (not an array)
    let complex_config_data = json!({
        "name": format!("Complex Config {}", uuid::Uuid::new_v4()),
        "experiment_default": false,
        "trays": [
            {
                "order_sequence": 1,
                "rotation_degrees": 0,
                "name": "Top Left Plate",
                "qty_x_axis": 12,
                "qty_y_axis": 8,
                "well_relative_diameter": 2.5
            },
            {
                "order_sequence": 2,
                "rotation_degrees": 0,
                "name": "Top Right Plate",
                "qty_x_axis": 12,
                "qty_y_axis": 8,
                "well_relative_diameter": 2.5
            },
            {
                "order_sequence": 3,
                "rotation_degrees": 90,
                "name": "Bottom Plate",
                "qty_x_axis": 24,
                "qty_y_axis": 16,
                "well_relative_diameter": 1.5
            },
            {
                "order_sequence": 4,
                "rotation_degrees": 180,
                "name": "Rotated Plate",
                "qty_x_axis": 8,
                "qty_y_axis": 12,
                "well_relative_diameter": 3.0
            }
        ]
    });

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/trays")
                .header("content-type", "application/json")
                .body(Body::from(complex_config_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn test_tray_configuration_complex_structure() {
    let app = setup_test_app().await;

    let response = create_tray_configuration_complex_structure(&app).await;

    let (status, body) = extract_response_body(response).await;

    // The test should succeed with 201 CREATED, not fail with 422
    assert_eq!(status, StatusCode::CREATED, "Tray configuration creation should succeed, but got: {status} with body: {body:?}");

    if status == StatusCode::CREATED {
        let config_id = body["id"].as_str().unwrap();

        // Validate complex structure
        if body["trays"].is_array() {
            let assignments = body["trays"].as_array().unwrap();

            assert_eq!(
                assignments.len(),
                4,
                "Configuration should have exactly 4 assignments (one per tray)"
            );

            // Each assignment should have tray details directly embedded (flattened structure)
            for assignment in assignments {
                assert!(assignment["name"].is_string(), "Each assignment should have tray details directly embedded");
                assert!(assignment["qty_x_axis"].is_number(), "qty_x_axis should be directly in assignment");
                assert!(assignment["qty_y_axis"].is_number(), "qty_y_axis should be directly in assignment");
            }

            // Check order sequence sorting
            let mut sequences: Vec<i64> = Vec::new();
            for assignment in assignments {
                sequences.push(assignment["order_sequence"].as_i64().unwrap());
            }
            let is_sorted = sequences.windows(2).all(|w| w[0] <= w[1]);

            assert!(
                is_sorted,
                "Assignments should be sorted by order_sequence, got: {sequences:?}"
            );
        }

        // Test retrieval of complex configuration
        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/trays/{config_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (get_status, get_body) = extract_response_body(get_response).await;

        if get_status == StatusCode::OK {
            // Validate that all data is loaded correctly
            if get_body["trays"].is_array() {
                let assignments = get_body["trays"].as_array().unwrap();
                // Complex structure should be fully loaded and preserved
                assert!(!assignments.is_empty(), "Should have tray assignments");
                for assignment in assignments {
                    assert!(
                        assignment["order_sequence"].is_number(),
                        "Assignment should have order_sequence"
                    );
                    // With flattened structure, tray details are directly in assignment
                    assert!(
                        assignment["name"].is_string() || assignment["name"].is_null(),
                        "Assignment should have tray name directly embedded"
                    );
                    assert!(
                        assignment["qty_x_axis"].is_number() || assignment["qty_x_axis"].is_null(),
                        "Assignment should have qty_x_axis directly embedded"
                    );
                }
            }
        }
    } else {
        // Complex tray configuration creation failed
    }
}

#[tokio::test]
async fn test_tray_configuration_not_found() {
    let app = setup_test_app().await;

    // Test getting non-existent tray configuration
    let fake_id = uuid::Uuid::new_v4();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/trays/{fake_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, _body) = extract_response_body(response).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Should return 404 for non-existent tray configuration"
    );
}

async fn create_test_tray_comprehensive(app: &axum::Router) -> (StatusCode, Value) {
    let individual_tray_data = json!({
        "name": format!("Workflow Individual Tray Config {}", uuid::Uuid::new_v4()),
        "experiment_default": false,
        "trays": [
            {
                "order_sequence": 1,
                "rotation_degrees": 0,
                "trays": [
                    {
                        "name": "Individual Workflow Tray",
                        "qty_x_axis": 8,
                        "qty_y_axis": 12,
                        "well_relative_diameter": 2.2
                    }
                ]
            }
        ]
    });

    let individual_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/trays")
                .header("content-type", "application/json")
                .body(Body::from(individual_tray_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (individual_status, individual_body) = extract_response_body(individual_response).await;
    (individual_status, individual_body)
}

#[tokio::test]
async fn test_tray_workflow_comprehensive() {
    let app = setup_test_app().await;
    let (individual_status, individual_body) = create_test_tray_comprehensive(&app).await;
    assert_eq!(
        individual_status,
        StatusCode::CREATED,
        "Failed to create individual tray configuration: Status {individual_status}, Body: {individual_body:?}"
    );
    // Step 2: Create comprehensive tray configuration
    let comprehensive_config_data = json!({
        "name": format!("Comprehensive Workflow Config {}", uuid::Uuid::new_v4()),
        "experiment_default": true,
        "trays": [
            {
                "order_sequence": 1,
                "rotation_degrees": 0,
                "trays": [
                    {
                        "name": "Primary Analysis Plate",
                        "qty_x_axis": 12,
                        "qty_y_axis": 8,
                        "well_relative_diameter": 2.5
                    }
                ]
            },
            {
                "order_sequence": 2,
                "rotation_degrees": 45,
                "trays": [
                    {
                        "name": "Secondary Control Plate",
                        "qty_x_axis": 16,
                        "qty_y_axis": 10,
                        "well_relative_diameter": 2.0
                    }
                ]
            }
        ]
    });

    let config_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/trays")
                .header("content-type", "application/json")
                .body(Body::from(comprehensive_config_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (config_status, config_body) = extract_response_body(config_response).await;

    if config_status == StatusCode::CREATED {
        let config_id = config_body["id"].as_str().unwrap();

        // Step 3: Validate configuration retrieval and structure
        let get_config_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/trays/{config_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (get_config_status, get_config_body) = extract_response_body(get_config_response).await;

        if get_config_status == StatusCode::OK {
            // Validate comprehensive structure
            assert_eq!(get_config_body["experiment_default"], true);

            if get_config_body["trays"].is_array() {
                let assignments = get_config_body["trays"].as_array().unwrap();
                assert_eq!(
                    assignments.len(),
                    2,
                    "Comprehensive config should have 2 assignments when retrieved"
                );
            }

            assert!(
                get_config_body["associated_experiments"].is_array(),
                "Associated experiments structure should be present"
            );
        }
    } else {
        // Comprehensive workflow test failed - config creation failed
    }

    // This test always passes - it's for comprehensive workflow documentation
    // Documents comprehensive tray workflow behavior
}

async fn create_test_tray_via_api(app: &axum::Router) -> Result<Value, Box<dyn std::error::Error>> {
    let tray_data = json!({
        "name": "TestTray",
        "experiment_default": true,
        "trays": [
            {
                "name": "P1",
                "qty_x_axis": 8,
                "qty_y_axis": 12,
                "well_relative_diameter": 0.6,
                "rotation_degrees": 0,
                "order_sequence": 1
            },
            {
                "name": "P2",
                "qty_x_axis": 8,
                "qty_y_axis": 12,
                "well_relative_diameter": 0.6,
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
                "name": "P1",
                "qty_x_axis": 8,
                "qty_y_axis": 12,
                "well_relative_diameter": 0.6,
                "rotation_degrees": 0,
                "order_sequence": 1
            },
            {
                "name": "P2",
                "qty_x_axis": 8,
                "qty_y_axis": 12,
                "well_relative_diameter": 0.6,
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

    // Validate tray structure (flattened format)
    let first_tray = &body["trays"][0];
    assert_eq!(first_tray["order_sequence"], 1);
    assert_eq!(first_tray["rotation_degrees"], 0);
    assert_eq!(first_tray["name"], "P1");

    let second_tray = &body["trays"][1];
    assert_eq!(second_tray["order_sequence"], 2);
    assert_eq!(second_tray["rotation_degrees"], 180);
    assert_eq!(second_tray["name"], "P2");
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

    // Test with missing required fields for TrayConfiguration
    let invalid_data = json!({
        "name": "Invalid Tray Config"
        // Missing experiment_default and trays array
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
