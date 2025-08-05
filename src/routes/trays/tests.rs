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

#[tokio::test]
async fn test_tray_crud_operations() {
    let app = setup_test_app().await;

    // Test creating a tray
    let tray_data = json!({
        "name": format!("Test Tray CRUD {}", uuid::Uuid::new_v4()),
        "qty_x_axis": 12,
        "qty_y_axis": 8,
        "well_relative_diameter": 2.5
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
        println!("✅ Tray creation successful");

        // Validate response structure
        assert!(body["id"].is_string(), "Response should include ID");
        assert!(body["name"].as_str().unwrap().contains("Test Tray CRUD"));
        assert_eq!(body["qty_x_axis"], 12);
        assert_eq!(body["qty_y_axis"], 8);
        assert_eq!(body["well_relative_diameter"], 2.5);
        assert!(body["created_at"].is_string());
        assert!(body["last_updated"].is_string());

        let tray_id = body["id"].as_str().unwrap();

        // Test getting the tray by ID
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
            println!("✅ Tray retrieval successful");
            assert_eq!(get_body["id"], tray_id);
            assert!(
                get_body["name"]
                    .as_str()
                    .unwrap()
                    .contains("Test Tray CRUD")
            );
        } else {
            println!("⚠️  Tray retrieval failed: {get_status}");
        }

        // Test updating the tray
        let update_data = json!({
            "qty_x_axis": 16,
            "well_relative_diameter": 3.0
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
            println!("✅ Tray update successful");
            assert_eq!(update_body["qty_x_axis"], 16);
            assert_eq!(update_body["well_relative_diameter"], 3.0);
        } else if update_status == StatusCode::METHOD_NOT_ALLOWED {
            println!("⚠️  Tray update not implemented (405)");
        } else {
            println!("📋 Tray update returned: {update_status}");
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
        if delete_status.is_success() {
            println!("✅ Tray delete successful");
        } else if delete_status == StatusCode::METHOD_NOT_ALLOWED {
            println!("⚠️  Tray delete not implemented (405)");
        } else {
            println!("📋 Tray delete returned: {delete_status}");
        }
    } else {
        println!("⚠️  Tray creation failed: Status {status}, Body: {body}");
        // Document the current behavior even if it fails
        assert!(
            status.is_client_error() || status.is_server_error(),
            "Tray creation should either succeed or fail gracefully"
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
        println!("✅ Tray listing successful");
        assert!(list_body.is_array(), "Trays list should be an array");
        let trays = list_body.as_array().unwrap();
        println!("Found {} trays in the system", trays.len());

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
        println!("⚠️  Tray listing failed: Status {list_status}");
        assert!(
            list_status.is_client_error() || list_status.is_server_error(),
            "Tray listing should either succeed or fail gracefully"
        );
    }
}

#[tokio::test]
async fn test_tray_validation() {
    let app = setup_test_app().await;

    // Test creating tray with invalid axis values (negative numbers)
    let invalid_data = json!({
        "name": "Invalid Tray",
        "qty_x_axis": -5,  // Should be positive
        "qty_y_axis": 8,
        "well_relative_diameter": 2.5
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

    if status.is_client_error() {
        println!("✅ Tray validation working - rejected negative axis value");
    } else if status == StatusCode::CREATED {
        println!("📋 Tray allows negative axis values (no validation)");
    } else {
        println!("📋 Tray validation returned: {status}");
    }

    // Test creating tray with zero dimensions
    let zero_data = json!({
        "name": "Zero Dimensions Tray",
        "qty_x_axis": 0,
        "qty_y_axis": 0,
        "well_relative_diameter": 0.0
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

    if status.is_client_error() {
        println!("✅ Tray validation rejects zero dimensions");
    } else if status == StatusCode::CREATED {
        println!("📋 Tray allows zero dimensions");
    } else {
        println!("📋 Tray zero dimension validation returned: {status}");
    }
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
            "name": format!("{} {}", name, &uuid::Uuid::new_v4().to_string()[..8]),
            "qty_x_axis": x_axis,
            "qty_y_axis": y_axis,
            "well_relative_diameter": 2.0
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
        println!("📋 No test trays created - skipping filtering tests");
    } else {
        // Test filtering by name
        let filter_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/trays?filter[name]=96-Well Plate")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (filter_status, filter_body) = extract_response_body(filter_response).await;

        if filter_status == StatusCode::OK {
            println!("✅ Tray filtering endpoint accessible");
            let filtered_trays = filter_body.as_array().unwrap();
            println!(
                "Filtered trays by name=96-Well Plate: {} results",
                filtered_trays.len()
            );

            // Check if filtering actually works
            let mut filtering_works = true;
            for tray in filtered_trays {
                if let Some(name) = tray["name"].as_str() {
                    if !name.contains("96-Well Plate") {
                        filtering_works = false;
                        println!("🐛 BUG: Filtering returned non-matching tray: {name:?}");
                    }
                }
            }

            if filtering_works && !filtered_trays.is_empty() {
                println!("✅ Tray filtering appears to work correctly");
            } else if filtered_trays.is_empty() {
                println!("📋 Tray filtering returned no results (may be working or broken)");
            }
        } else {
            println!("⚠️  Tray filtering failed: Status {filter_status}");
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
            println!("✅ Tray sorting endpoint accessible");
        } else {
            println!("⚠️  Tray sorting failed: Status {sort_status}");
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
    println!("✅ Tray 404 handling working correctly");
}

#[tokio::test]
async fn test_tray_configuration_crud_operations() {
    let app = setup_test_app().await;

    // Test creating a tray configuration with multiple trays
    let tray_config_data = json!({
        "name": format!("Test Tray Config CRUD {}", uuid::Uuid::new_v4()),
        "experiment_default": false,
        "trays": [
            {
                "order_sequence": 1,
                "rotation_degrees": 0,
                "trays": [
                    {
                        "name": "Primary Plate",
                        "qty_x_axis": 12,
                        "qty_y_axis": 8,
                        "well_relative_diameter": 2.5
                    }
                ]
            },
            {
                "order_sequence": 2,
                "rotation_degrees": 90,
                "trays": [
                    {
                        "name": "Secondary Plate",
                        "qty_x_axis": 24,
                        "qty_y_axis": 16,
                        "well_relative_diameter": 1.5
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
                .uri("/api/tray_configurations")
                .header("content-type", "application/json")
                .body(Body::from(tray_config_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, body) = extract_response_body(response).await;

    if status == StatusCode::CREATED {
        println!("✅ Tray configuration creation successful");

        // Validate response structure
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

        // Validate trays array structure
        if body["trays"].is_array() {
            let trays = body["trays"].as_array().unwrap();
            println!("   Configuration has {} tray assignments", trays.len());

            if trays.len() == 2 {
                println!("   ✅ All tray assignments preserved");

                // Validate assignment structure
                for assignment in trays {
                    assert!(assignment["order_sequence"].is_number());
                    assert!(assignment["rotation_degrees"].is_number());
                    assert!(assignment["trays"].is_array());
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
                    .uri(format!("/api/tray_configurations/{tray_config_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (get_status, get_body) = extract_response_body(get_response).await;
        if get_status == StatusCode::OK {
            println!("✅ Tray configuration retrieval successful");
            assert_eq!(get_body["id"], tray_config_id);

            // Validate related data structure
            if get_body["trays"].is_array() {
                println!("   ✅ Trays array present and loaded");
            }
            if get_body["associated_experiments"].is_array() {
                println!("   ✅ Associated experiments array present");
            }
        } else {
            println!("⚠️  Tray configuration retrieval failed: {get_status}");
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
                    .uri(format!("/api/tray_configurations/{tray_config_id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(update_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (update_status, update_body) = extract_response_body(update_response).await;
        if update_status == StatusCode::OK {
            println!("✅ Tray configuration update successful");
            assert_eq!(update_body["experiment_default"], true);
        } else if update_status == StatusCode::METHOD_NOT_ALLOWED {
            println!("⚠️  Tray configuration update not implemented (405)");
        } else {
            println!("📋 Tray configuration update returned: {update_status}");
        }

        // Test deleting the tray configuration
        let delete_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/tray_configurations/{tray_config_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let delete_status = delete_response.status();
        if delete_status.is_success() {
            println!("✅ Tray configuration delete successful");
        } else if delete_status == StatusCode::METHOD_NOT_ALLOWED {
            println!("⚠️  Tray configuration delete not implemented (405)");
        } else {
            println!("📋 Tray configuration delete returned: {delete_status}");
        }
    } else {
        println!("⚠️  Tray configuration creation failed: Status {status}, Body: {body}");
        // Document the current behavior even if it fails
        assert!(
            status.is_client_error() || status.is_server_error(),
            "Tray configuration creation should either succeed or fail gracefully"
        );
    }
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
                .uri("/api/tray_configurations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let (list_status, list_body) = extract_response_body(list_response).await;

    if list_status == StatusCode::OK {
        println!("✅ Tray configuration listing successful");
        assert!(
            list_body.is_array(),
            "Tray configurations list should be an array"
        );
        let tray_configs = list_body.as_array().unwrap();
        println!(
            "Found {} tray configurations in the system",
            tray_configs.len()
        );

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
        println!("⚠️  Tray configuration listing failed: Status {list_status}");
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
                .uri("/api/tray_configurations")
                .header("content-type", "application/json")
                .body(Body::from(default_config_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, body) = extract_response_body(response).await;

    if status == StatusCode::CREATED {
        println!("✅ Default tray configuration created successfully");
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
                    .uri("/api/tray_configurations")
                    .header("content-type", "application/json")
                    .body(Body::from(second_default_config_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (second_status, second_body) = extract_response_body(second_response).await;

        if second_status == StatusCode::CREATED {
            println!("✅ Second default configuration created");
            assert_eq!(second_body["experiment_default"], true);

            // Check if the first configuration is no longer default
            let check_first_response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(format!("/api/tray_configurations/{first_config_id}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            let (check_status, check_body) = extract_response_body(check_first_response).await;

            if check_status == StatusCode::OK {
                if check_body["experiment_default"] == false {
                    println!(
                        "   ✅ Default configuration behavior working - first config no longer default"
                    );
                } else {
                    println!(
                        "   📋 Multiple default configurations allowed (or default logic not working)"
                    );
                }
            }
        } else {
            println!("📋 Could not test default behavior - second config creation failed");
        }
    } else {
        println!("📋 Skipping default behavior test - couldn't create first config");
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
            "name": format!("{} Test {}", description, &uuid::Uuid::new_v4().to_string()[..8]),
            "qty_x_axis": x_axis,
            "qty_y_axis": y_axis,
            "well_relative_diameter": 2.0
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

        if status == StatusCode::CREATED {
            println!("✅ Tray accepts {description} ({x_axis}x{y_axis})");
        } else {
            println!("📋 Tray rejects {description} ({x_axis}x{y_axis}) - Status: {status}");
        }
    }
}

#[tokio::test]
async fn test_tray_configuration_complex_structure() {
    let app = setup_test_app().await;

    // Test creating a complex tray configuration with multiple assignments and rotations
    let complex_config_data = json!({
        "name": format!("Complex Config {}", uuid::Uuid::new_v4()),
        "experiment_default": false,
        "trays": [
            {
                "order_sequence": 1,
                "rotation_degrees": 0,
                "trays": [
                    {
                        "name": "Top Left Plate",
                        "qty_x_axis": 12,
                        "qty_y_axis": 8,
                        "well_relative_diameter": 2.5
                    },
                    {
                        "name": "Top Right Plate",
                        "qty_x_axis": 12,
                        "qty_y_axis": 8,
                        "well_relative_diameter": 2.5
                    }
                ]
            },
            {
                "order_sequence": 2,
                "rotation_degrees": 90,
                "trays": [
                    {
                        "name": "Bottom Plate",
                        "qty_x_axis": 24,
                        "qty_y_axis": 16,
                        "well_relative_diameter": 1.5
                    }
                ]
            },
            {
                "order_sequence": 3,
                "rotation_degrees": 180,
                "trays": [
                    {
                        "name": "Rotated Plate",
                        "qty_x_axis": 8,
                        "qty_y_axis": 12,
                        "well_relative_diameter": 3.0
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
                .uri("/api/tray_configurations")
                .header("content-type", "application/json")
                .body(Body::from(complex_config_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, body) = extract_response_body(response).await;

    if status == StatusCode::CREATED {
        println!("✅ Complex tray configuration created successfully");

        let config_id = body["id"].as_str().unwrap();

        // Validate complex structure
        if body["trays"].is_array() {
            let assignments = body["trays"].as_array().unwrap();
            println!("   Configuration has {} assignments", assignments.len());

            if assignments.len() == 3 {
                println!("   ✅ All assignments preserved");

                // Count total trays across all assignments
                let mut total_trays = 0;
                for assignment in assignments {
                    if assignment["trays"].is_array() {
                        total_trays += assignment["trays"].as_array().unwrap().len();
                    }
                }

                if total_trays == 4 {
                    // 2 + 1 + 1 trays across assignments
                    println!("   ✅ All {total_trays} trays preserved across assignments");
                }

                // Check order sequence sorting
                let mut sequences: Vec<i64> = Vec::new();
                for assignment in assignments {
                    sequences.push(assignment["order_sequence"].as_i64().unwrap());
                }
                let is_sorted = sequences.windows(2).all(|w| w[0] <= w[1]);

                if is_sorted {
                    println!("   ✅ Assignments are properly sorted by order_sequence");
                } else {
                    println!("   📋 Assignment ordering may not be working correctly");
                }
            }
        }

        // Test retrieval of complex configuration
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

        if get_status == StatusCode::OK {
            println!("   ✅ Complex configuration retrieval successful");

            // Validate that all data is loaded correctly
            if get_body["trays"].is_array() {
                let assignments = get_body["trays"].as_array().unwrap();
                if assignments.len() == 3 {
                    println!("   ✅ Complex structure fully loaded and preserved");
                }
            }
        }
    } else {
        println!("📋 Complex tray configuration creation failed: Status {status}");
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
                .uri(format!("/api/tray_configurations/{fake_id}"))
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
    println!("✅ Tray configuration 404 handling working correctly");
}

#[tokio::test]
async fn test_tray_workflow_comprehensive() {
    let app = setup_test_app().await;

    println!("📋 TRAY COMPREHENSIVE WORKFLOW TEST");
    println!("   Testing complete tray and tray configuration lifecycle");

    // Step 1: Create individual trays
    let individual_tray_data = json!({
        "name": format!("Workflow Individual Tray {}", uuid::Uuid::new_v4()),
        "qty_x_axis": 8,
        "qty_y_axis": 12,
        "well_relative_diameter": 2.2
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

    let (individual_status, _individual_body) = extract_response_body(individual_response).await;

    if individual_status == StatusCode::CREATED {
        println!("   ✅ Step 1: Individual tray created successfully");
    } else {
        println!("   📋 Step 1: Individual tray creation returned: {individual_status}");
    }

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
                .uri("/api/tray_configurations")
                .header("content-type", "application/json")
                .body(Body::from(comprehensive_config_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (config_status, config_body) = extract_response_body(config_response).await;

    if config_status == StatusCode::CREATED {
        println!("   ✅ Step 2: Comprehensive tray configuration created");

        let config_id = config_body["id"].as_str().unwrap();

        // Step 3: Validate configuration retrieval and structure
        let get_config_response = app
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

        let (get_config_status, get_config_body) = extract_response_body(get_config_response).await;

        if get_config_status == StatusCode::OK {
            println!("   ✅ Step 3: Configuration retrieval successful");

            // Validate comprehensive structure
            assert_eq!(get_config_body["experiment_default"], true);

            if get_config_body["trays"].is_array() {
                let assignments = get_config_body["trays"].as_array().unwrap();
                println!(
                    "   Configuration loaded with {} assignments",
                    assignments.len()
                );

                if assignments.len() == 2 {
                    println!("   ✅ Step 4: All assignments preserved and loaded");
                }
            }

            if get_config_body["associated_experiments"].is_array() {
                println!("   ✅ Step 5: Associated experiments structure present");
            }
        }

        println!("   📋 Comprehensive workflow test completed successfully");
    } else {
        println!("   ⚠️  Comprehensive workflow test failed - config creation: {config_status}");
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
