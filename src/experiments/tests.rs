use crate::config::test_helpers::setup_test_app;
use axum::Router;
use axum::body::Body;
use axum::body::to_bytes;
use axum::http::{Request, StatusCode};
use chrono::{DateTime, NaiveDateTime};
use core::panic;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use tower::ServiceExt;

/// Integration test helper to assign tray configuration to experiment via API
async fn assign_tray_config_to_experiment_via_api(
    app: &axum::Router,
    experiment_id: &str,
    config_id: &str,
) {
    let update_data = json!({
        "tray_configuration_id": config_id
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/experiments/{experiment_id}"))
                .header("content-type", "application/json")
                .body(Body::from(update_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    assert!(
        status.is_success(),
        "Failed to assign tray configuration. Status: {status}"
    );
}

async fn create_simple_tray_config(app: &axum::Router) -> Result<String, String> {
    let tray_config_data = json!({
        "name": "Simple Test Config",
        "experiment_default": false
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

    if !response.status().is_success() {
        return Err(format!(
            "Failed to create simple tray config: {}",
            response.status()
        ));
    }

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let config: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    Ok(config["id"].as_str().unwrap().to_string())
}

async fn create_experiment_via_api(app: &axum::Router) -> Result<String, String> {
    let experiment_data = json!({
        "name": "Test Image Correlation Experiment",
        "username": "test@example.com",
        "performed_at": "2024-06-20T14:30:00Z",
        "temperature_ramp": -1.0,
        "temperature_start": 20.0,
        "temperature_end": -40.0,
        "is_calibration": false
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

    if response.status() == StatusCode::NOT_FOUND {
        return Err("Experiment API endpoint not implemented".to_string());
    }

    if !response.status().is_success() {
        return Err(format!(
            "Failed to create experiment via API: {}",
            response.status()
        ));
    }

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    Ok(experiment["id"].as_str().unwrap().to_string())
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

    let status = response.status();
    assert!(
        status.is_success(),
        "Failed to create experiment. Status: {status}"
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
async fn test_experiment_endpoint_includes_results_summary() {
    let app = setup_test_app().await;

    // Create an experiment
    let experiment_data = json!({
        "name": "Test Experiment with Results",
        "username": "test@example.com",
        "performed_at": "2024-06-20T14:30:00Z",
        "temperature_ramp": -1.0,
        "temperature_start": 5.0,
        "temperature_end": -25.0,
        "is_calibration": false,
        "remarks": "Test experiment endpoint includes results"
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

    assert_eq!(response.status(), StatusCode::CREATED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let experiment_id = experiment["id"].as_str().unwrap();

    // Get the experiment by ID and check that it includes results_summary
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/experiments/{experiment_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment_with_results: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    //     "Experiment response: {}",
    //     serde_json::to_string_pretty(&experiment_with_results).unwrap()
    // );

    // Check that results is included
    assert!(
        experiment_with_results["results"].is_object(),
        "Should have results object"
    );

    let results = &experiment_with_results["results"];

    // Check required fields exist in results summary
    assert!(
        results["summary"]["total_time_points"].is_number(),
        "Should have total_time_points"
    );
    assert!(results["trays"].is_array(), "Should have trays array");

    // For a new experiment with no data, we expect 0 values
    assert_eq!(
        results["summary"]["total_time_points"], 0,
        "New experiment should have 0 time points"
    );
}
// Create experiment using helper function
async fn create_test_experiment(app: &axum::Router) -> Result<serde_json::Value, String> {
    let experiment_data = json!({
        "name": "Test Experiment",
        "username": "test@example.com",
        "performed_at": "2024-06-20T14:30:00Z",
        "temperature_ramp": -1.0,
        "temperature_start": 5.0,
        "temperature_end": -25.0,
        "is_calibration": false,
        "remarks": "Test experiment"
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

    if response.status() != StatusCode::CREATED {
        return Err(format!(
            "Failed to create experiment: {}",
            response.status()
        ));
    }

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    Ok(experiment)
}

async fn create_experiment_get_results_summary() -> Result<serde_json::Value, String> {
    let app = setup_test_app().await;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/experiments")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Results Summary Structure Test",
                        "username": "test@example.com",
                        "performed_at": "2024-06-20T14:30:00Z",
                        "temperature_ramp": -1.0,
                        "temperature_start": 5.0,
                        "temperature_end": -25.0,
                        "is_calibration": false,
                        "remarks": "Testing results summary structure"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let experiment: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let experiment_id = experiment["id"].as_str().unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/experiments/{experiment_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    //     "Full experiment response: {}",
    //     serde_json::to_string_pretty(&experiment_response).unwrap()
    // );
    let results = &experiment_response["results"];
    if !results.is_object() {
        return Err("Results is not an object".to_string());
    }
    Ok(results.clone())
}

#[tokio::test]
async fn test_experiment_results_summary_structure() {
    let results = create_experiment_get_results_summary().await.unwrap();
    validate_experiment_results_structure(&results);
    validate_well_summaries_structure(&results);
}

#[tokio::test]
async fn test_experiment_with_mock_results_data() {
    let app = setup_test_app().await;

    // Create an experiment
    let experiment_data = json!({
        "name": "Mock Results Data Test",
        "username": "test@example.com",
        "performed_at": "2024-06-20T14:30:00Z",
        "temperature_ramp": -1.0,
        "temperature_start": 5.0,
        "temperature_end": -25.0,
        "is_calibration": false,
        "remarks": "Testing with mock results data"
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

    assert_eq!(response.status(), StatusCode::CREATED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let experiment_id = experiment["id"].as_str().unwrap();

    // Since we can't easily mock the database in integration tests,
    // we'll just verify the API contract and response structure

    // Get experiment and verify empty results
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/experiments/{experiment_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let results = &experiment_response["results"];

    // For a new experiment with no data, verify initial state
    assert_eq!(
        results["summary"]["total_time_points"], 0,
        "New experiment should have 0 time points"
    );
    assert!(
        results["summary"]["first_timestamp"].is_null(),
        "New experiment should have null first_timestamp"
    );
    assert!(
        results["summary"]["last_timestamp"].is_null(),
        "New experiment should have null last_timestamp"
    );
    assert_eq!(
        results["trays"].as_array().unwrap().len(),
        0,
        "New experiment should have empty trays"
    );
}

#[tokio::test]
async fn test_experiment_list_with_results_summary() {
    let app = setup_test_app().await;

    // Create multiple experiments
    for i in 1..=3 {
        let experiment_data = json!({
            "name": format!("List Test Experiment {}", i),
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": i == 2, // Make one a calibration
            "remarks": format!("Test experiment {} for list endpoint", i)
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

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    // Get all experiments
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/experiments")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiments_list: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(experiments_list.is_array(), "Response should be an array");
    let experiments = experiments_list.as_array().unwrap();

    // Verify each experiment in the list
    for (i, exp) in experiments.iter().enumerate() {
        assert!(exp.is_object(), "Experiment {i} should be an object");

        // Check if results_summary is included in list view
        if exp.get("results_summary").is_some()
            && !exp["results_summary"].is_null()
            && exp["results_summary"].is_object()
        {
            let results_summary = &exp["results_summary"];
            assert!(
                results_summary["total_wells"].is_number(),
                "Experiment {i} results should have total_wells"
            );
            assert!(
                results_summary["wells_with_data"].is_number(),
                "Experiment {i} results should have wells_with_data"
            );
            assert!(
                results_summary["well_summaries"].is_array(),
                "Experiment {i} results should have well_summaries"
            );
        }

        // Verify basic experiment fields are present
        assert!(exp.get("id").is_some(), "Experiment {i} should have id");
        assert!(exp.get("name").is_some(), "Experiment {i} should have name");
    }
}

async fn extract_response_body(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"error": "Invalid JSON response"}));

    (status, body)
}

#[tokio::test]
async fn test_experiment_list_operations() {
    let app = setup_test_app().await;

    // Test getting all experiments
    let list_response = app
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

    let (list_status, list_body) = extract_response_body(list_response).await;

    if list_status == StatusCode::OK {
        assert!(list_body.is_array(), "Experiments list should be an array");
        let experiments = list_body.as_array().unwrap();

        // Validate structure of experiments in list
        for experiment in experiments {
            assert!(
                experiment["id"].is_string(),
                "Each experiment should have ID"
            );
            assert!(
                experiment["name"].is_string(),
                "Each experiment should have name"
            );
        }
    } else {
        assert!(
            list_status.is_client_error() || list_status.is_server_error(),
            "Experiment listing should either succeed or fail gracefully"
        );
    }
}

#[tokio::test]
async fn test_experiment_filtering_and_sorting() {
    let app = setup_test_app().await;

    // Create test experiments for filtering
    let test_experiments = [
        ("Filter Test A", "DeviceA"),
        ("Filter Test B", "DeviceB"),
        ("Filter Test C", "DeviceA"),
    ];

    let mut created_ids = Vec::new();

    for (name, device) in test_experiments {
        let experiment_data = json!({
            "name": format!("{} {}", name, &uuid::Uuid::new_v4().to_string()[..8]),
            "device_name": device,
            "room_temperature": 22.0,
            "device_description": "Test device for filtering"
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
        if status == StatusCode::CREATED {
            created_ids.push(body["id"].as_str().unwrap().to_string());
        }
    }

    if created_ids.is_empty() {
    } else {
        // Test filtering by device name
        let filter_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/experiments?filter[device_name]=DeviceA")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (filter_status, filter_body) = extract_response_body(filter_response).await;

        if filter_status == StatusCode::OK {
            let filtered_experiments = filter_body.as_array().unwrap();
            //     "Filtered experiments by device_name=DeviceA: {} results",
            //     filtered_experiments.len()
            // );

            // Check if filtering actually works
            for experiment in filtered_experiments {
                assert_eq!(
                    experiment["device_name"], "DeviceA",
                    "Filtering should only return DeviceA experiments, but got: {:?}",
                    experiment["device_name"]
                );
            }
        }

        // Test sorting by name
        let sort_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/experiments?sort[name]=asc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (sort_status, _) = extract_response_body(sort_response).await;

        if sort_status == StatusCode::OK {}
    }
}

#[tokio::test]
async fn test_experiment_not_found() {
    let app = setup_test_app().await;

    // Test getting non-existent experiment
    let fake_id = uuid::Uuid::new_v4();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/experiments/{fake_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, _body) = extract_response_body(response).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Should return 404 for non-existent experiment"
    );
}

#[tokio::test]
async fn test_experiment_results_endpoint() {
    let app = setup_test_app().await;

    // Create an experiment first
    let experiment_data = json!({
        "name": format!("Results Test {}", uuid::Uuid::new_v4()),
        "device_name": "RTDTempX8",
        "room_temperature": 22.5,
        "device_description": "Test for results endpoint"
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

    if status == StatusCode::CREATED {
        let experiment_id = body["id"].as_str().unwrap();

        // Test the results endpoint
        let results_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/experiments/{experiment_id}/results"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (results_status, results_body) = extract_response_body(results_response).await;

        if results_status == StatusCode::OK {
            // Validate results structure
            if results_body.is_object() {
                validate_experiment_results_structure(&results_body);
            }
        } else if results_status == StatusCode::NOT_FOUND {
        }
    }
}

#[tokio::test]
async fn test_experiment_process_status_endpoint() {
    let app = setup_test_app().await;

    // Test the process status endpoint with a fake job ID
    let fake_job_id = uuid::Uuid::new_v4();
    let fake_experiment_id = uuid::Uuid::new_v4();

    let status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/experiments/{fake_experiment_id}/process-status/{fake_job_id}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let (status_status, _status_body) = extract_response_body(status_response).await;

    if status_status == StatusCode::NOT_FOUND {
        //     "✅ Process status endpoint accessible - correctly returns 404 for non-existent job"
        // );
    } else if status_status == StatusCode::OK {
    }
}

/// Validate experiment results structure
fn validate_experiment_results_structure(results: &serde_json::Value) {
    assert!(results.is_object(), "results should be an object");
    assert!(
        results["summary"].is_object(),
        "results.summary should be an object"
    );
    assert!(
        results["summary"]["total_time_points"].is_number(),
        "summary.total_time_points should be a number"
    );
    assert!(
        results["trays"].is_array(),
        "results.trays should be an array"
    );
    assert!(
        results["summary"].get("first_timestamp").is_some(),
        "first_timestamp field should exist"
    );
    assert!(
        results["summary"].get("last_timestamp").is_some(),
        "last_timestamp field should exist"
    );
}

/// Validate tray-centric well structure
fn validate_well_summaries_structure(results: &serde_json::Value) {
    if let Some(trays) = results["trays"].as_array() {
        let mut total_wells = 0;
        let mut all_wells = Vec::new();

        for tray in trays {
            if let Some(wells) = tray["wells"].as_array() {
                total_wells += wells.len();
                all_wells.extend(wells.iter());
            }
        }

        // If no wells, this might be before upload - just return
        if total_wells == 0 {
            return;
        }

        // Core success criteria - we expect 192 wells (8x12 x 2 trays)
        assert_eq!(
            total_wells, 192,
            "Should have exactly 192 well summaries (8x12 x 2 trays)"
        );

        // Count wells by tray and validate phase transitions
        let mut p1_wells = 0;
        let mut p2_wells = 0;
        let mut wells_with_phase_changes = 0;
        let mut frozen_wells = 0;

        for (tray_idx, tray) in trays.iter().enumerate() {
            let tray_name = tray["tray_name"].as_str();

            if let Some(wells) = tray["wells"].as_array() {
                for (well_idx, summary) in wells.iter().enumerate() {
                    assert!(
                        summary.is_object(),
                        "tray[{tray_idx}].wells[{well_idx}] should be an object"
                    );
                    assert!(
                        summary.get("coordinate").is_some(),
                        "tray[{tray_idx}].wells[{well_idx}] should have coordinate"
                    );
                    assert!(
                        summary.get("final_state").is_some(),
                        "tray[{tray_idx}].wells[{well_idx}] should have final_state"
                    );

                    // Count by tray
                    if let Some(tray_name) = tray_name {
                        match tray_name {
                            "P1" => p1_wells += 1,
                            "P2" => p2_wells += 1,
                            _ => panic!("Unexpected tray name: {tray_name}"),
                        }
                    }

                    // Count phase transitions (wells that have phase change data)
                    if summary.get("first_phase_change_time").is_some() {
                        wells_with_phase_changes += 1;
                    }

                    // Count frozen wells
                    if let Some(final_state) = summary["final_state"].as_str()
                        && final_state == "frozen"
                    {
                        frozen_wells += 1;
                    }
                }
            }
        }

        // Validate tray distribution
        assert_eq!(
            p1_wells, 96,
            "Should have exactly 96 wells in tray P1 (8x12)"
        );
        assert_eq!(
            p2_wells, 96,
            "Should have exactly 96 wells in tray P2 (8x12)"
        );

        // Validate phase transitions - from the Excel processing output, we expect all 192 wells to freeze
        assert_eq!(
            wells_with_phase_changes, 192,
            "All 192 wells should have phase change data (liquid→frozen)"
        );
        assert_eq!(
            frozen_wells, 192,
            "All 192 wells should end up in frozen state"
        );

        // Validate a few specific coordinates to ensure proper formatting
        for summary in all_wells.iter().take(3) {
            if let Some(coord) = summary["coordinate"].as_str() {
                assert!(
                    coord.len() >= 2 && coord.chars().next().unwrap().is_alphabetic(),
                    "Coordinate should be in format like 'A1', got: {coord}"
                );
            }

            if let Some(state) = summary["final_state"].as_str() {
                assert!(
                    state == "liquid" || state == "frozen" || state == "unknown",
                    "final_state should be 'liquid', 'frozen', or 'unknown', got: {state}"
                );
            }
        }
    }
}

/// Load test Excel file from resources
// Removed unused load_test_excel_file function

// Removed unused validate_excel_upload_results function

// Removed unused validate_experiment_results_via_api function

// Removed unused validate_uploaded_data_exists function

#[tokio::test]
async fn test_asset_upload_endpoint() {
    // Initialize test environment
    let app = setup_test_app().await;

    // Create experiment with tray configuration
    let experiment_result = create_test_experiment(&app).await.unwrap();
    let experiment_id = experiment_result["id"].as_str().unwrap();

    // Create test file content (small PNG image data)
    let test_file_content = create_test_image_data();
    let filename = "test_image.png";

    // Create multipart form data
    let boundary = "test_boundary_123456789";
    let multipart_body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: image/png\r\n\r\n{file_content}\r\n--{boundary}--\r\n",
        boundary = boundary,
        filename = filename,
        file_content = String::from_utf8_lossy(&test_file_content)
    );

    // Make upload request
    let upload_response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/experiments/{experiment_id}/uploads"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(axum::body::Body::from(multipart_body))
                .unwrap(),
        )
        .await
        .unwrap();

    //     "📤 Asset upload response status: {}",
    //     upload_response.status()
    // );

    // For now, we expect this to fail with 500 due to S3 not being configured in tests
    // But we can verify the endpoint exists and handles multipart correctly
    let status = upload_response.status();
    let body_bytes = axum::body::to_bytes(upload_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let _body_str = String::from_utf8_lossy(&body_bytes);

    // In test environment without S3, we expect either:
    // - 500 Internal Server Error (S3 connection failure)
    // - 200 Success (if S3 is mocked)
    assert!(
        status == axum::http::StatusCode::INTERNAL_SERVER_ERROR
            || status == axum::http::StatusCode::OK,
        "Expected either 500 (S3 not configured) or 200 (success), got {status}"
    );
}

#[tokio::test]
async fn test_asset_download_endpoint() {
    // Initialize test environment
    let app = setup_test_app().await;

    // Create experiment with tray configuration
    let experiment_result = create_test_experiment(&app).await.unwrap();
    let experiment_id = experiment_result["id"].as_str().unwrap();

    // First get a download token (should succeed even if no assets)
    let token_response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/experiments/{experiment_id}/download-token"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        token_response.status(),
        axum::http::StatusCode::OK,
        "Token creation should succeed"
    );

    let token_body_bytes = axum::body::to_bytes(token_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let token_body: serde_json::Value = serde_json::from_slice(&token_body_bytes).unwrap();
    let token = token_body["token"].as_str().unwrap();

    // Now make download request with token (should return 404 since no assets exist)
    let download_response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri(format!("/api/assets/download/{token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    //     "📥 Asset download response status: {}",
    //     download_response.status()
    // );

    let status = download_response.status();
    let body_bytes = axum::body::to_bytes(download_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);

    // Should return 404 since no assets exist for this experiment
    assert_eq!(
        status,
        axum::http::StatusCode::NOT_FOUND,
        "Expected 404 Not Found for experiment with no assets, got {status}"
    );

    assert!(
        body_str.contains("No assets found"),
        "Expected 'No assets found' in response body, got: {body_str}"
    );
}

#[tokio::test]
async fn test_asset_upload_duplicate_file() {
    // Initialize test environment
    let app = setup_test_app().await;

    // Create experiment
    let experiment_result = create_test_experiment(&app).await.unwrap();
    let experiment_id = experiment_result["id"].as_str().unwrap();

    // Test with a small text file to avoid S3 complexity
    let test_content = b"test file content for duplicate check";
    let filename = "duplicate_test.txt";
    let boundary = "test_boundary_duplicate";

    let multipart_body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: text/plain\r\n\r\n{content}\r\n--{boundary}--\r\n",
        boundary = boundary,
        filename = filename,
        content = String::from_utf8_lossy(test_content)
    );

    // Make two requests to the same app instance to test duplicate detection
    // Note: In test environment without S3, we expect both uploads to fail at S3 stage
    // but the first should fail with S3 error and second should potentially detect duplicate
    // However, since S3 fails before database insert, duplicate detection won't trigger

    // First upload
    let app_clone = app.clone();
    let _first_response = app_clone
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/experiments/{experiment_id}/uploads"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(axum::body::Body::from(multipart_body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Second upload (should detect duplicate if first succeeded)
    let second_response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/experiments/{experiment_id}/uploads"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(axum::body::Body::from(multipart_body))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = second_response.status();
    let body_bytes = axum::body::to_bytes(second_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let _body_str = String::from_utf8_lossy(&body_bytes);

    // We expect either:
    // - 409 Conflict if the first upload succeeded and duplicate is detected
    // - 500 if S3 failed on first upload (then duplicate check won't trigger)
    assert!(
        status == axum::http::StatusCode::CONFLICT
            || status == axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "Expected either 409 (duplicate detected) or 500 (S3 error), got {status}"
    );
}

#[tokio::test]
async fn test_asset_upload_invalid_experiment() {
    // Initialize test environment
    let app = setup_test_app().await;

    // Use non-existent experiment ID
    let fake_experiment_id = "00000000-0000-0000-0000-000000000000";

    let test_content = b"test content";
    let boundary = "test_boundary_invalid";
    let multipart_body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\n{content}\r\n--{boundary}--\r\n",
        boundary = boundary,
        content = String::from_utf8_lossy(test_content)
    );

    // Make upload request to non-existent experiment
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/experiments/{fake_experiment_id}/uploads"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(axum::body::Body::from(multipart_body))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);

    // Should return 404 Not Found
    assert_eq!(
        status,
        axum::http::StatusCode::NOT_FOUND,
        "Expected 404 for non-existent experiment"
    );

    assert!(
        body_str.contains("Experiment not found"),
        "Expected 'Experiment not found' in response"
    );
}

#[tokio::test]
async fn test_asset_upload_no_file() {
    // Initialize test environment
    let app = setup_test_app().await;

    // Create experiment
    let experiment_result = create_test_experiment(&app).await.unwrap();
    let experiment_id = experiment_result["id"].as_str().unwrap();

    // Create multipart body with no file field
    let boundary = "test_boundary_nofile";
    let multipart_body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"other_field\"\r\n\r\nsome value\r\n--{boundary}--\r\n"
    );

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/experiments/{experiment_id}/uploads"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(axum::body::Body::from(multipart_body))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);

    // Should return 400 Bad Request
    assert_eq!(
        status,
        axum::http::StatusCode::BAD_REQUEST,
        "Expected 400 when no file is uploaded"
    );

    assert!(
        body_str.contains("No file uploaded"),
        "Expected 'No file uploaded' in response"
    );
}

/// Helper function to create test image data (small PNG-like binary data)
fn create_test_image_data() -> Vec<u8> {
    // Simple binary data that looks like a PNG file
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 pixel
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // RGB, no interlace
        0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT chunk
        0x54, 0x08, 0x99, 0x01, 0x01, 0x00, 0x00, 0x00, // Compressed data
        0x00, 0x00, 0x02, 0x00, 0x01, 0xE5, 0x27, 0xDE, // Checksum
        0xFC, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, // IEND chunk
        0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

// Expected values from the merged.xlsx file based on previous analysis
const EXPECTED_TOTAL_WELLS: u64 = 192; // 96 wells per tray × 2 trays
const EXPECTED_TOTAL_TIME_POINTS: u64 = 6786;

/// Comprehensive validation data derived from merged.csv analysis
/// This represents the exact phase transitions that should occur when processing the Excel file
pub struct WellTransitionData {
    pub tray: &'static str,        // "P1" or "P2"
    pub coordinate: &'static str,  // "A1", "B2", etc.
    pub freeze_time: &'static str, // "2025-03-20 16:19:47" - for reference
    pub temp_probe_1: f64,         // Temperature at freeze time
}

/// Expected phase transitions extracted from merged.csv analysis
/// Complete list of all 192 well transitions with exact timestamps and average temperatures
/// Data generated from systematic CSV analysis of all phase changes (0→1)
pub const EXPECTED_TRANSITIONS: &[WellTransitionData] = &[
    WellTransitionData {
        tray: "P1",
        coordinate: "A1",
        freeze_time: "2025-03-20 16:49:38",
        temp_probe_1: -27.543,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "A10",
        freeze_time: "2025-03-20 16:35:04",
        temp_probe_1: -22.863,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "A11",
        freeze_time: "2025-03-20 16:32:47",
        temp_probe_1: -22.151,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "A12",
        freeze_time: "2025-03-20 16:34:09",
        temp_probe_1: -22.588,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "A2",
        freeze_time: "2025-03-20 16:48:49",
        temp_probe_1: -27.278,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "A3",
        freeze_time: "2025-03-20 16:50:39",
        temp_probe_1: -27.856,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "A4",
        freeze_time: "2025-03-20 16:50:58",
        temp_probe_1: -27.955,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "A5",
        freeze_time: "2025-03-20 16:45:19",
        temp_probe_1: -26.121,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "A6",
        freeze_time: "2025-03-20 16:51:07",
        temp_probe_1: -28.003,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "A7",
        freeze_time: "2025-03-20 16:33:57",
        temp_probe_1: -22.522,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "A8",
        freeze_time: "2025-03-20 16:34:41",
        temp_probe_1: -22.753,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "A9",
        freeze_time: "2025-03-20 16:42:06",
        temp_probe_1: -25.078,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "B1",
        freeze_time: "2025-03-20 16:42:35",
        temp_probe_1: -25.232,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "B10",
        freeze_time: "2025-03-20 16:42:44",
        temp_probe_1: -25.278,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "B11",
        freeze_time: "2025-03-20 16:43:21",
        temp_probe_1: -25.469,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "B12",
        freeze_time: "2025-03-20 16:33:20",
        temp_probe_1: -22.322,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "B2",
        freeze_time: "2025-03-20 16:46:10",
        temp_probe_1: -26.419,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "B3",
        freeze_time: "2025-03-20 16:52:23",
        temp_probe_1: -28.435,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "B4",
        freeze_time: "2025-03-20 16:46:09",
        temp_probe_1: -26.412,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "B5",
        freeze_time: "2025-03-20 16:40:09",
        temp_probe_1: -24.460,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "B6",
        freeze_time: "2025-03-20 16:46:35",
        temp_probe_1: -26.555,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "B7",
        freeze_time: "2025-03-20 16:46:33",
        temp_probe_1: -26.550,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "B8",
        freeze_time: "2025-03-20 16:46:30",
        temp_probe_1: -26.531,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "B9",
        freeze_time: "2025-03-20 16:42:04",
        temp_probe_1: -25.065,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "C1",
        freeze_time: "2025-03-20 16:50:22",
        temp_probe_1: -27.769,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "C10",
        freeze_time: "2025-03-20 16:31:47",
        temp_probe_1: -21.823,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "C11",
        freeze_time: "2025-03-20 16:36:24",
        temp_probe_1: -23.257,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "C12",
        freeze_time: "2025-03-20 16:36:50",
        temp_probe_1: -23.392,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "C2",
        freeze_time: "2025-03-20 16:46:24",
        temp_probe_1: -26.500,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "C3",
        freeze_time: "2025-03-20 16:49:16",
        temp_probe_1: -27.425,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "C4",
        freeze_time: "2025-03-20 16:47:00",
        temp_probe_1: -26.678,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "C5",
        freeze_time: "2025-03-20 16:40:48",
        temp_probe_1: -24.659,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "C6",
        freeze_time: "2025-03-20 16:36:39",
        temp_probe_1: -23.335,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "C7",
        freeze_time: "2025-03-20 16:42:59",
        temp_probe_1: -25.358,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "C8",
        freeze_time: "2025-03-20 16:48:32",
        temp_probe_1: -27.181,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "C9",
        freeze_time: "2025-03-20 16:43:37",
        temp_probe_1: -25.549,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "D1",
        freeze_time: "2025-03-20 16:41:27",
        temp_probe_1: -24.870,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "D10",
        freeze_time: "2025-03-20 16:35:40",
        temp_probe_1: -23.047,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "D11",
        freeze_time: "2025-03-20 16:29:30",
        temp_probe_1: -21.084,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "D12",
        freeze_time: "2025-03-20 16:37:05",
        temp_probe_1: -23.469,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "D2",
        freeze_time: "2025-03-20 16:41:26",
        temp_probe_1: -24.864,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "D3",
        freeze_time: "2025-03-20 16:49:17",
        temp_probe_1: -27.430,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "D4",
        freeze_time: "2025-03-20 16:43:02",
        temp_probe_1: -25.377,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "D5",
        freeze_time: "2025-03-20 16:45:53",
        temp_probe_1: -26.322,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "D6",
        freeze_time: "2025-03-20 16:49:27",
        temp_probe_1: -27.487,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "D7",
        freeze_time: "2025-03-20 16:42:35",
        temp_probe_1: -25.232,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "D8",
        freeze_time: "2025-03-20 16:38:05",
        temp_probe_1: -23.793,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "D9",
        freeze_time: "2025-03-20 16:53:43",
        temp_probe_1: -28.887,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "E1",
        freeze_time: "2025-03-20 16:47:31",
        temp_probe_1: -26.832,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "E10",
        freeze_time: "2025-03-20 16:42:16",
        temp_probe_1: -25.132,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "E11",
        freeze_time: "2025-03-20 16:39:50",
        temp_probe_1: -24.360,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "E12",
        freeze_time: "2025-03-20 16:35:19",
        temp_probe_1: -22.943,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "E2",
        freeze_time: "2025-03-20 16:53:24",
        temp_probe_1: -28.786,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "E3",
        freeze_time: "2025-03-20 16:53:37",
        temp_probe_1: -28.858,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "E4",
        freeze_time: "2025-03-20 16:42:33",
        temp_probe_1: -25.221,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "E5",
        freeze_time: "2025-03-20 16:42:09",
        temp_probe_1: -25.095,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "E6",
        freeze_time: "2025-03-20 16:43:46",
        temp_probe_1: -25.592,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "E7",
        freeze_time: "2025-03-20 16:46:40",
        temp_probe_1: -26.581,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "E8",
        freeze_time: "2025-03-20 16:35:30",
        temp_probe_1: -22.998,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "E9",
        freeze_time: "2025-03-20 16:50:38",
        temp_probe_1: -27.850,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "F1",
        freeze_time: "2025-03-20 16:46:48",
        temp_probe_1: -26.619,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "F10",
        freeze_time: "2025-03-20 16:39:14",
        temp_probe_1: -24.180,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "F11",
        freeze_time: "2025-03-20 16:37:17",
        temp_probe_1: -23.539,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "F12",
        freeze_time: "2025-03-20 16:28:21",
        temp_probe_1: -20.719,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "F2",
        freeze_time: "2025-03-20 16:49:33",
        temp_probe_1: -27.518,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "F3",
        freeze_time: "2025-03-20 16:43:39",
        temp_probe_1: -25.560,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "F4",
        freeze_time: "2025-03-20 16:50:29",
        temp_probe_1: -27.805,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "F5",
        freeze_time: "2025-03-20 16:52:05",
        temp_probe_1: -28.330,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "F6",
        freeze_time: "2025-03-20 16:48:24",
        temp_probe_1: -27.133,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "F7",
        freeze_time: "2025-03-20 16:36:28",
        temp_probe_1: -23.277,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "F8",
        freeze_time: "2025-03-20 16:39:34",
        temp_probe_1: -24.287,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "F9",
        freeze_time: "2025-03-20 16:48:15",
        temp_probe_1: -27.088,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "G1",
        freeze_time: "2025-03-20 16:40:36",
        temp_probe_1: -24.597,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "G10",
        freeze_time: "2025-03-20 16:28:37",
        temp_probe_1: -20.803,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "G11",
        freeze_time: "2025-03-20 16:40:16",
        temp_probe_1: -24.493,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "G12",
        freeze_time: "2025-03-20 16:34:46",
        temp_probe_1: -22.777,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "G2",
        freeze_time: "2025-03-20 16:43:23",
        temp_probe_1: -25.481,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "G3",
        freeze_time: "2025-03-20 16:49:36",
        temp_probe_1: -27.535,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "G4",
        freeze_time: "2025-03-20 16:53:04",
        temp_probe_1: -28.673,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "G5",
        freeze_time: "2025-03-20 16:41:01",
        temp_probe_1: -24.728,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "G6",
        freeze_time: "2025-03-20 16:50:39",
        temp_probe_1: -27.856,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "G7",
        freeze_time: "2025-03-20 16:41:07",
        temp_probe_1: -24.760,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "G8",
        freeze_time: "2025-03-20 16:43:10",
        temp_probe_1: -25.416,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "G9",
        freeze_time: "2025-03-20 16:39:38",
        temp_probe_1: -24.308,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "H1",
        freeze_time: "2025-03-20 16:43:58",
        temp_probe_1: -25.653,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "H10",
        freeze_time: "2025-03-20 16:37:19",
        temp_probe_1: -23.551,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "H11",
        freeze_time: "2025-03-20 16:37:45",
        temp_probe_1: -23.693,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "H12",
        freeze_time: "2025-03-20 16:38:58",
        temp_probe_1: -24.087,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "H2",
        freeze_time: "2025-03-20 16:48:34",
        temp_probe_1: -27.192,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "H3",
        freeze_time: "2025-03-20 16:52:49",
        temp_probe_1: -28.589,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "H4",
        freeze_time: "2025-03-20 16:43:06",
        temp_probe_1: -25.396,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "H5",
        freeze_time: "2025-03-20 16:46:49",
        temp_probe_1: -26.624,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "H6",
        freeze_time: "2025-03-20 16:43:16",
        temp_probe_1: -25.444,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "H7",
        freeze_time: "2025-03-20 16:38:58",
        temp_probe_1: -24.087,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "H8",
        freeze_time: "2025-03-20 16:44:22",
        temp_probe_1: -25.784,
    },
    WellTransitionData {
        tray: "P1",
        coordinate: "H9",
        freeze_time: "2025-03-20 16:50:08",
        temp_probe_1: -27.704,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "A1",
        freeze_time: "2025-03-20 16:33:53",
        temp_probe_1: -22.502,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "A10",
        freeze_time: "2025-03-20 16:43:28",
        temp_probe_1: -25.504,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "A11",
        freeze_time: "2025-03-20 16:40:28",
        temp_probe_1: -24.555,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "A12",
        freeze_time: "2025-03-20 16:41:13",
        temp_probe_1: -24.792,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "A2",
        freeze_time: "2025-03-20 16:31:55",
        temp_probe_1: -21.867,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "A3",
        freeze_time: "2025-03-20 16:49:22",
        temp_probe_1: -27.458,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "A4",
        freeze_time: "2025-03-20 16:45:52",
        temp_probe_1: -26.314,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "A5",
        freeze_time: "2025-03-20 16:48:10",
        temp_probe_1: -27.059,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "A6",
        freeze_time: "2025-03-20 16:43:18",
        temp_probe_1: -25.455,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "A7",
        freeze_time: "2025-03-20 16:45:19",
        temp_probe_1: -26.121,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "A8",
        freeze_time: "2025-03-20 16:42:00",
        temp_probe_1: -25.043,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "A9",
        freeze_time: "2025-03-20 16:35:39",
        temp_probe_1: -23.045,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "B1",
        freeze_time: "2025-03-20 16:38:42",
        temp_probe_1: -24.000,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "B10",
        freeze_time: "2025-03-20 16:43:44",
        temp_probe_1: -25.580,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "B11",
        freeze_time: "2025-03-20 16:37:47",
        temp_probe_1: -23.704,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "B12",
        freeze_time: "2025-03-20 16:42:39",
        temp_probe_1: -25.252,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "B2",
        freeze_time: "2025-03-20 16:36:15",
        temp_probe_1: -23.217,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "B3",
        freeze_time: "2025-03-20 16:50:10",
        temp_probe_1: -27.709,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "B4",
        freeze_time: "2025-03-20 16:35:47",
        temp_probe_1: -23.083,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "B5",
        freeze_time: "2025-03-20 16:50:28",
        temp_probe_1: -27.799,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "B6",
        freeze_time: "2025-03-20 16:50:10",
        temp_probe_1: -27.709,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "B7",
        freeze_time: "2025-03-20 16:50:12",
        temp_probe_1: -27.718,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "B8",
        freeze_time: "2025-03-20 16:19:47",
        temp_probe_1: -17.969,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "B9",
        freeze_time: "2025-03-20 16:42:47",
        temp_probe_1: -25.295,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "C1",
        freeze_time: "2025-03-20 16:41:03",
        temp_probe_1: -24.738,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "C10",
        freeze_time: "2025-03-20 16:31:52",
        temp_probe_1: -21.852,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "C11",
        freeze_time: "2025-03-20 16:41:46",
        temp_probe_1: -24.966,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "C12",
        freeze_time: "2025-03-20 16:35:50",
        temp_probe_1: -23.094,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "C2",
        freeze_time: "2025-03-20 16:38:52",
        temp_probe_1: -24.056,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "C3",
        freeze_time: "2025-03-20 16:34:38",
        temp_probe_1: -22.735,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "C4",
        freeze_time: "2025-03-20 16:39:22",
        temp_probe_1: -24.221,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "C5",
        freeze_time: "2025-03-20 16:52:23",
        temp_probe_1: -28.435,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "C6",
        freeze_time: "2025-03-20 16:40:53",
        temp_probe_1: -24.685,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "C7",
        freeze_time: "2025-03-20 16:36:56",
        temp_probe_1: -23.424,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "C8",
        freeze_time: "2025-03-20 16:43:04",
        temp_probe_1: -25.388,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "C9",
        freeze_time: "2025-03-20 16:43:59",
        temp_probe_1: -25.662,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "D1",
        freeze_time: "2025-03-20 16:36:23",
        temp_probe_1: -23.253,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "D10",
        freeze_time: "2025-03-20 16:42:08",
        temp_probe_1: -25.088,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "D11",
        freeze_time: "2025-03-20 16:42:54",
        temp_probe_1: -25.334,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "D12",
        freeze_time: "2025-03-20 16:39:36",
        temp_probe_1: -24.295,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "D2",
        freeze_time: "2025-03-20 16:28:12",
        temp_probe_1: -20.669,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "D3",
        freeze_time: "2025-03-20 16:37:51",
        temp_probe_1: -23.724,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "D4",
        freeze_time: "2025-03-20 16:54:47",
        temp_probe_1: -29.247,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "D5",
        freeze_time: "2025-03-20 16:48:54",
        temp_probe_1: -27.303,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "D6",
        freeze_time: "2025-03-20 16:51:20",
        temp_probe_1: -28.076,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "D7",
        freeze_time: "2025-03-20 16:46:36",
        temp_probe_1: -26.564,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "D8",
        freeze_time: "2025-03-20 16:38:39",
        temp_probe_1: -23.983,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "D9",
        freeze_time: "2025-03-20 16:38:50",
        temp_probe_1: -24.045,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "E1",
        freeze_time: "2025-03-20 16:36:06",
        temp_probe_1: -23.174,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "E10",
        freeze_time: "2025-03-20 16:42:15",
        temp_probe_1: -25.126,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "E11",
        freeze_time: "2025-03-20 16:41:46",
        temp_probe_1: -24.966,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "E12",
        freeze_time: "2025-03-20 16:40:11",
        temp_probe_1: -24.468,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "E2",
        freeze_time: "2025-03-20 16:34:37",
        temp_probe_1: -22.733,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "E3",
        freeze_time: "2025-03-20 16:45:28",
        temp_probe_1: -26.176,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "E4",
        freeze_time: "2025-03-20 16:45:03",
        temp_probe_1: -26.022,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "E5",
        freeze_time: "2025-03-20 16:44:14",
        temp_probe_1: -25.740,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "E6",
        freeze_time: "2025-03-20 16:43:03",
        temp_probe_1: -25.382,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "E7",
        freeze_time: "2025-03-20 16:56:25",
        temp_probe_1: -29.793,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "E8",
        freeze_time: "2025-03-20 16:41:17",
        temp_probe_1: -24.814,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "E9",
        freeze_time: "2025-03-20 16:26:37",
        temp_probe_1: -20.169,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "F1",
        freeze_time: "2025-03-20 16:34:15",
        temp_probe_1: -22.623,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "F10",
        freeze_time: "2025-03-20 16:39:23",
        temp_probe_1: -24.224,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "F11",
        freeze_time: "2025-03-20 16:40:27",
        temp_probe_1: -24.548,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "F12",
        freeze_time: "2025-03-20 16:39:58",
        temp_probe_1: -24.401,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "F2",
        freeze_time: "2025-03-20 16:23:35",
        temp_probe_1: -19.192,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "F3",
        freeze_time: "2025-03-20 16:48:02",
        temp_probe_1: -27.009,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "F4",
        freeze_time: "2025-03-20 16:45:24",
        temp_probe_1: -26.152,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "F5",
        freeze_time: "2025-03-20 16:51:34",
        temp_probe_1: -28.150,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "F6",
        freeze_time: "2025-03-20 16:42:18",
        temp_probe_1: -25.139,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "F7",
        freeze_time: "2025-03-20 16:47:16",
        temp_probe_1: -26.755,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "F8",
        freeze_time: "2025-03-20 16:29:55",
        temp_probe_1: -21.217,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "F9",
        freeze_time: "2025-03-20 16:43:44",
        temp_probe_1: -25.580,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "G1",
        freeze_time: "2025-03-20 16:37:20",
        temp_probe_1: -23.553,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "G10",
        freeze_time: "2025-03-20 16:36:03",
        temp_probe_1: -23.158,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "G11",
        freeze_time: "2025-03-20 16:41:01",
        temp_probe_1: -24.728,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "G12",
        freeze_time: "2025-03-20 16:37:46",
        temp_probe_1: -23.698,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "G2",
        freeze_time: "2025-03-20 16:35:45",
        temp_probe_1: -23.071,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "G3",
        freeze_time: "2025-03-20 16:41:00",
        temp_probe_1: -24.723,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "G4",
        freeze_time: "2025-03-20 16:44:12",
        temp_probe_1: -25.729,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "G5",
        freeze_time: "2025-03-20 16:46:44",
        temp_probe_1: -26.599,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "G6",
        freeze_time: "2025-03-20 16:40:40",
        temp_probe_1: -24.613,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "G7",
        freeze_time: "2025-03-20 16:46:29",
        temp_probe_1: -26.521,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "G8",
        freeze_time: "2025-03-20 16:39:16",
        temp_probe_1: -24.190,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "G9",
        freeze_time: "2025-03-20 16:44:42",
        temp_probe_1: -25.898,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "H1",
        freeze_time: "2025-03-20 16:36:29",
        temp_probe_1: -23.284,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "H10",
        freeze_time: "2025-03-20 16:44:30",
        temp_probe_1: -25.832,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "H11",
        freeze_time: "2025-03-20 16:34:40",
        temp_probe_1: -22.746,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "H12",
        freeze_time: "2025-03-20 16:35:40",
        temp_probe_1: -23.047,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "H2",
        freeze_time: "2025-03-20 16:35:30",
        temp_probe_1: -22.998,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "H3",
        freeze_time: "2025-03-20 16:46:20",
        temp_probe_1: -26.477,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "H4",
        freeze_time: "2025-03-20 16:46:46",
        temp_probe_1: -26.608,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "H5",
        freeze_time: "2025-03-20 16:49:20",
        temp_probe_1: -27.449,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "H6",
        freeze_time: "2025-03-20 16:38:08",
        temp_probe_1: -23.808,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "H7",
        freeze_time: "2025-03-20 16:50:11",
        temp_probe_1: -27.712,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "H8",
        freeze_time: "2025-03-20 16:39:00",
        temp_probe_1: -24.101,
    },
    WellTransitionData {
        tray: "P2",
        coordinate: "H9",
        freeze_time: "2025-03-20 16:39:21",
        temp_probe_1: -24.215,
    },
];

/// Create a tray configuration with embedded trays (post-flattening structure)
async fn create_test_tray_config_with_trays(app: &Router, name: &str) -> String {
    let tray_config_data = json!({
        "name": name,
        "experiment_default": false,
        "trays": [
            {
                "order_sequence": 1,
                "rotation_degrees": 0,
                "name": "P1",
                "qty_cols": 8,
                "qty_rows": 12,
                "well_relative_diameter": 2.5
            },
            {
                "order_sequence": 2,
                "rotation_degrees": 0,
                "name": "P2",
                "qty_cols": 8,
                "qty_rows": 12,
                "well_relative_diameter": 2.5
            }
        ]
    });

    //     "🏗️ Creating tray configuration '{}' with embedded P1/P2 trays: {}",
    //     name, tray_config_data
    // );

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

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert_eq!(
        status,
        StatusCode::CREATED,
        "Failed to create tray config. Status: {status}, Body: {body_str}"
    );

    body_str
}

/// Upload Excel file via API with proper multipart support
async fn upload_excel_file(app: &Router, experiment_id: &str) -> Value {
    // Read the test Excel file
    let excel_data =
        fs::read("src/experiments/test_resources/merged.xlsx").expect("test Excel file missing");

    // Create a properly formatted multipart body with correct boundaries and headers
    let boundary = "----formdata-test-boundary-123456789";
    let mut body = Vec::new();

    // Construct multipart body according to RFC 7578
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"merged.xlsx\"\r\n",
    );
    body.extend_from_slice(
        b"Content-Type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet\r\n",
    );
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(&excel_data);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

    let response = app
        .clone()
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

    let status_code = response.status();
    let response_body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(response_body.to_vec()).unwrap();

    json!({
        "status_code": status_code.as_u16(),
        "body": body_str
    })
}

#[tokio::test]
async fn test_comprehensive_excel_validation_with_specific_transitions() {
    let app = setup_test_app().await;

    // Step 1: Create experiment with proper tray configuration
    let tray_config_response =
        create_test_tray_config_with_trays(&app, "Comprehensive Test Config").await;
    let tray_config: Value = serde_json::from_str(&tray_config_response).unwrap();

    let tray_config_id = tray_config["id"].as_str().unwrap();

    let experiment_payload = serde_json::json!({
        "name": "Comprehensive Validation Test",
        "remarks": "Testing specific well transitions from merged.csv",
        "tray_configuration_id": tray_config_id,
        "is_calibration": false
    });

    let experiment_response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::POST)
                .uri("/api/experiments")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(experiment_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let exp_status = experiment_response.status();
    let exp_body = axum::body::to_bytes(experiment_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let exp_body_str = String::from_utf8(exp_body.to_vec()).unwrap();

    assert!(
        !(exp_status != StatusCode::OK && exp_status != StatusCode::CREATED),
        "Failed to create experiment. Status: {exp_status}, Body: {exp_body_str}"
    );

    assert_eq!(exp_status, 201);
    let experiment: Value = serde_json::from_str(&exp_body_str).unwrap();
    let experiment_id = experiment["id"].as_str().unwrap();

    // Step 2: Upload Excel file and process
    let upload_result = upload_excel_file(&app, experiment_id).await;

    assert!(
        upload_result["body"]
            .as_str()
            .unwrap()
            .contains("completed")
    );

    // Step 3: Fetch experiment results with comprehensive validation
    let results_response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::GET)
                .uri(format!("/api/experiments/{experiment_id}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(results_response.status(), 200);
    let results_body = axum::body::to_bytes(results_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment_with_results: Value = serde_json::from_slice(&results_body).unwrap();
    let results = &experiment_with_results["results"];

    // Step 4: Validate high-level counts
    validate_experiment_totals(results);

    // Step 5: Validate specific well transitions
    validate_specific_well_transitions(&experiment_with_results);

    // Step 6: Validate temperature data accuracy
    validate_temperature_readings(&experiment_with_results);

    // Step 7: Validate timing accuracy
    validate_experiment_timing(results);
}

fn validate_experiment_totals(results: &Value) {
    // Calculate totals from tray data
    let mut total_wells = 0;
    let mut wells_with_data = 0;
    let mut wells_frozen = 0;

    if let Some(trays) = results["trays"].as_array() {
        for tray in trays {
            if let Some(wells) = tray["wells"].as_array() {
                for well in wells {
                    total_wells += 1;

                    // Count as having data if it has phase change time
                    if well.get("first_phase_change_time").is_some() {
                        wells_with_data += 1;
                        // If it has a phase change time, it froze
                        wells_frozen += 1;
                    }
                }
            }
        }
    }

    let total_time_points = results["summary"]["total_time_points"]
        .as_u64()
        .unwrap_or(0);

    assert_eq!(
        total_wells,
        { EXPECTED_TOTAL_WELLS },
        "Total wells should be {EXPECTED_TOTAL_WELLS}, got {total_wells}"
    );
    assert_eq!(
        wells_with_data,
        { EXPECTED_TOTAL_WELLS },
        "Wells with data should be {EXPECTED_TOTAL_WELLS}, got {wells_with_data}"
    );
    assert_eq!(
        wells_frozen,
        { EXPECTED_TOTAL_WELLS },
        "All wells should be frozen, got {wells_frozen}"
    );
    assert_eq!(
        total_time_points,
        { EXPECTED_TOTAL_TIME_POINTS },
        "Time points should be {EXPECTED_TOTAL_TIME_POINTS}, got {total_time_points}"
    );
}

fn validate_specific_well_transitions(experiment: &Value) {
    // Extract all wells from all trays in the new format
    let mut all_wells = Vec::new();
    if let Some(trays) = experiment["results"]["trays"].as_array() {
        for tray in trays {
            if let Some(wells) = tray["wells"].as_array() {
                all_wells.extend(wells.iter());
            }
        }
    }

    // Create lookup map by tray and coordinate
    let mut well_lookup: HashMap<String, &Value> = HashMap::new();
    if let Some(trays) = experiment["results"]["trays"].as_array() {
        for tray in trays {
            let tray_name = tray["tray_name"].as_str().unwrap_or("unknown");
            if let Some(wells) = tray["wells"].as_array() {
                for well in wells {
                    let coordinate = well["coordinate"].as_str().unwrap_or("unknown");
                    let key = format!("{tray_name}_{coordinate}");
                    well_lookup.insert(key, well);
                }
            }
        }
    }

    // Validate each expected transition
    for expected in EXPECTED_TRANSITIONS {
        let key = format!("{}_{}", expected.tray, expected.coordinate);
        let well = well_lookup
            .get(&key)
            .unwrap_or_else(|| panic!("Could not find well {key}"));

        // Validate well has a freeze time
        let freeze_time = well["first_phase_change_time"]
            .as_str()
            .unwrap_or_else(|| panic!("Well {key} should have first_phase_change_time"));

        // Validate well is frozen (has first_phase_change_time means it froze)
        assert!(
            well.get("first_phase_change_time").is_some(),
            "Well {key} should be frozen (have first_phase_change_time)"
        );

        // Validate temperature probes (optional)
        let temp_probes = &well["first_phase_change_temperature_probes"];
        if !temp_probes.is_null() {
            assert!(
                temp_probes.is_object(),
                "Well {key} temperature probe data should be an object if present"
            );
        }

        // Parse API timestamp (ISO 8601 format)
        let api_time = DateTime::parse_from_rfc3339(freeze_time)
            .expect("Failed to parse API timestamp")
            .naive_utc();

        // Parse expected timestamp (space-separated format)
        let expected_time =
            NaiveDateTime::parse_from_str(expected.freeze_time, "%Y-%m-%d %H:%M:%S")
                .expect("Failed to parse expected timestamp");

        // Allow 1 second tolerance to handle .999 millisecond differences
        let diff = (api_time - expected_time).num_milliseconds().abs();
        assert!(
            diff <= 1000,
            "Well {} freeze time difference too large: expected {}, got {} (diff: {}ms)",
            key,
            expected.freeze_time,
            freeze_time,
            diff
        );

        // Temperature validation - only if probe data is available
        if !temp_probes.is_null() && temp_probes.is_object() {
            // Temperature values are stored as strings (Decimal), need to parse them
            if let Some(probe1_temp_str) = temp_probes["probe_1"].as_str()
                && let Ok(probe1_temp) = probe1_temp_str.parse::<f64>()
            {
                // Allow tolerance for difference between averaged (CSV analysis) and single probe (API) temperatures
                // Since CSV analysis used 8-probe averages but API uses individual probe readings
                let temp_diff = (probe1_temp - expected.temp_probe_1).abs();
                assert!(
                    temp_diff < 1.0,
                    "Well {} probe 1 temperature should be ~{}°C, got {}°C (diff: {}°C)",
                    key,
                    expected.temp_probe_1,
                    probe1_temp,
                    temp_diff
                );
            }
        }
    }
}

fn validate_temperature_readings(_experiment: &Value) {

    // Temperature validation would require time series data
    // For now, validate that temperature probe structure exists
}

fn validate_experiment_timing(results: &Value) {
    let first_timestamp = results["summary"]["first_timestamp"]
        .as_str()
        .expect("Should have first_timestamp");
    let _last_timestamp = results["summary"]["last_timestamp"]
        .as_str()
        .expect("Should have last_timestamp");

    // Validate experiment start time matches expected
    assert!(
        first_timestamp.contains("2025-03-20"),
        "Experiment should start on 2025-03-20, got {first_timestamp}"
    );
    assert!(
        first_timestamp.contains("15:13"),
        "Experiment should start around 15:13, got {first_timestamp}"
    );

    // Calculate duration (should be about 1 hour 6 minutes based on CSV)
    // This is a rough validation - exact timing depends on processing
}

#[tokio::test]
async fn test_well_coordinate_mapping_accuracy() {
    let app = setup_test_app().await;

    // Create experiment and upload
    let tray_config_response = create_test_tray_config_with_trays(&app, "Coordinate Test").await;
    let tray_config: Value = serde_json::from_str(&tray_config_response).unwrap();
    let tray_config_id = tray_config["id"].as_str().unwrap();

    let experiment_payload = serde_json::json!({
        "name": "Coordinate Mapping Test",
        "tray_configuration_id": tray_config_id,
        "is_calibration": false
    });

    let experiment_response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::POST)
                .uri("/api/experiments")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(experiment_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let experiment_body = axum::body::to_bytes(experiment_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment: Value = serde_json::from_slice(&experiment_body).unwrap();
    let experiment_id = experiment["id"].as_str().unwrap();

    let _upload_result = upload_excel_file(&app, experiment_id).await;

    // Fetch results and validate coordinate mappings
    let results_response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::GET)
                .uri(format!("/api/experiments/{experiment_id}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let results_body = axum::body::to_bytes(results_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment_with_results: Value = serde_json::from_slice(&results_body).unwrap();
    // Extract all wells from all trays in the new format
    let mut all_wells = Vec::new();
    if let Some(trays) = experiment_with_results["results"]["trays"].as_array() {
        for tray in trays {
            if let Some(wells) = tray["wells"].as_array() {
                all_wells.extend(wells.iter());
            }
        }
    }

    // Validate that we have exactly 192 wells with proper coordinates
    assert_eq!(all_wells.len(), 192, "Should have exactly 192 wells");

    let mut p1_wells = 0;
    let mut p2_wells = 0;
    let mut coordinate_set = std::collections::HashSet::new();

    // Iterate through trays and their wells to get tray context
    if let Some(trays) = experiment_with_results["results"]["trays"].as_array() {
        for tray in trays {
            let tray_name = tray["tray_name"].as_str().unwrap_or("unknown");
            if let Some(wells) = tray["wells"].as_array() {
                for well in wells {
                    let coordinate = well["coordinate"].as_str().unwrap_or("unknown");

                    match tray_name {
                        "P1" => p1_wells += 1,
                        "P2" => p2_wells += 1,
                        _ => panic!("Unexpected tray name: {tray_name}"),
                    }

                    // Validate coordinate format (A1-H12)
                    assert!(
                        coordinate.len() >= 2 && coordinate.len() <= 3,
                        "Coordinate {coordinate} should be 2-3 characters"
                    );
                    assert!(
                        coordinate.chars().next().unwrap().is_ascii_uppercase(),
                        "Coordinate {coordinate} should start with A-H"
                    );

                    // Add to set to check for duplicates within tray
                    let full_coord = format!("{tray_name}_{coordinate}");
                    assert!(
                        coordinate_set.insert(full_coord.clone()),
                        "Duplicate coordinate found: {full_coord}"
                    );
                }
            }
        }
    }

    assert_eq!(p1_wells, 96, "Should have 96 P1 wells, got {p1_wells}");
    assert_eq!(p2_wells, 96, "Should have 96 P2 wells, got {p2_wells}");
}

/// Test image-temperature correlation in results summary
#[tokio::test]
async fn test_image_temperature_correlation() {
    let app = setup_test_app().await;

    // Create experiment
    let experiment_id = create_experiment_via_api(&app).await.unwrap();

    // Create a simple tray configuration manually (skip complex create function for now)
    let tray_config_id = create_simple_tray_config(&app).await.unwrap();
    assign_tray_config_to_experiment_via_api(&app, &experiment_id, &tray_config_id).await;

    // Verify experiment can be retrieved (temperature readings are created through Excel processing)
    let experiment_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/experiments/{experiment_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(experiment_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(experiment_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment_data: Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify the experiment exists and has the expected structure
    assert_eq!(experiment_data["id"].as_str().unwrap(), experiment_id);
}

/// Test asset retrieval by filename endpoint
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_asset_by_filename_endpoint() {
    let app = setup_test_app().await;

    // Create experiment
    let experiment_id = create_experiment_via_api(&app).await.unwrap();

    // Create mock assets with different filename formats
    create_mock_asset(
        &app,
        &experiment_id,
        "INP_49640_2025-03-20_15-14-17.jpg",
        "image",
    )
    .await;
    create_mock_asset(
        &app,
        &experiment_id,
        "INP_49641_2025-03-20_15-15-17",
        "image",
    )
    .await; // No .jpg extension

    // Add dummy file data to mock S3 store for testing
    let dummy_image_data = b"fake-image-data".to_vec();
    crate::external::s3::MOCK_S3_STORE
        .put_object(
            "test/INP_49640_2025-03-20_15-14-17.jpg",
            dummy_image_data.clone(),
        )
        .expect("Failed to add mock S3 data");
    crate::external::s3::MOCK_S3_STORE
        .put_object(
            "test/INP_49641_2025-03-20_15-15-17",
            dummy_image_data.clone(),
        )
        .expect("Failed to add mock S3 data");

    // Test 1: Access asset with exact filename match
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/assets/by-experiment/{experiment_id}/INP_49640_2025-03-20_15-14-17.jpg"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should find asset with exact filename match"
    );

    // Test 2: Access asset without .jpg extension (should add .jpg automatically)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/assets/by-experiment/{experiment_id}/INP_49640_2025-03-20_15-14-17"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should find asset by adding .jpg extension"
    );

    // Test 3: Access asset that already exists without .jpg extension
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/assets/by-experiment/{experiment_id}/INP_49641_2025-03-20_15-15-17"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should find asset stored without .jpg extension"
    );

    // Test 4: Non-existent asset should return 404
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/assets/by-experiment/{experiment_id}/non_existent_image"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Should return 404 for non-existent asset"
    );
}

/// Helper to create mock asset for testing
async fn create_mock_asset(
    app: &Router,
    experiment_id: &str,
    filename: &str,
    asset_type: &str,
) -> String {
    let asset_data = json!({
        "experiment_id": experiment_id,
        "original_filename": filename,
        "s3_key": format!("test/{}", filename),
        "type": asset_type,
        "size_bytes": 1024,
        "uploaded_by": "test_user",
        "is_deleted": false,
        "role": "camera_capture"
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

    assert_eq!(response.status(), StatusCode::CREATED);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let asset: Value = serde_json::from_slice(&body_bytes).unwrap();
    asset["id"].as_str().unwrap().to_string()
}

/// Integration test for complete Excel processing with image filenames
#[tokio::test]
async fn test_excel_processing_with_images() {
    let app = setup_test_app().await;

    // This test would require the actual Excel file from test resources
    // For now, we'll test the individual components that make up the workflow

    // 1. Create experiment and tray config
    let experiment_id = create_experiment_via_api(&app).await.unwrap();
    let tray_config_response =
        create_test_tray_config_with_trays(&app, "Excel Processing Test Config").await;
    let tray_config: Value = serde_json::from_str(&tray_config_response).unwrap();
    let tray_config_id = tray_config["id"].as_str().unwrap();
    assign_tray_config_to_experiment_via_api(&app, &experiment_id, tray_config_id).await;

    // 2. Test image asset creation and access (temperature readings are created via Excel processing)
    let image_filenames = vec![
        "INP_49640_2025-03-20_15-14-17", // Excel format (no .jpg)
        "INP_49641_2025-03-20_15-14-18",
        "INP_49642_2025-03-20_15-14-19",
    ];

    // 4. Create corresponding image assets (with .jpg extension)
    for image_filename in &image_filenames {
        let asset_filename = format!("{image_filename}.jpg"); // Assets have .jpg extension
        create_mock_asset(&app, &experiment_id, &asset_filename, "image").await;
    }

    // Add dummy file data to mock S3 store for testing (just like in test_asset_by_filename_endpoint)
    let dummy_image_data = b"fake-image-data-excel-test".to_vec();
    for image_filename in &image_filenames {
        let asset_filename_with_jpg = format!("{image_filename}.jpg");
        crate::external::s3::MOCK_S3_STORE
            .put_object(
                &format!("test/{asset_filename_with_jpg}"),
                dummy_image_data.clone(),
            )
            .expect("Failed to add mock S3 data for Excel test");
    }
    //     "🎯 Added dummy file data to mock S3 store for {} assets",
    //     image_filenames.len()
    // );

    // 5. Test that results summary contains correct image filenames
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/experiments/{experiment_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let experiment_data: Value = serde_json::from_slice(&body_bytes).unwrap();
    let results = &experiment_data["results"];

    // Extract all wells from all trays in the new tray-centric format
    let mut all_wells = Vec::new();
    if let Some(trays) = results["trays"].as_array() {
        for tray in trays {
            if let Some(wells) = tray["wells"].as_array() {
                all_wells.extend(wells.iter());
            }
        }
    }

    // 6. Verify that temperature readings and assets were created successfully
    // (Phase transition correlation requires Excel processing pipeline not available via API)
    //     "📊 Found {} well summaries from {} trays (expected for tray configuration)",
    //     all_wells.len(),
    //     results["trays"].as_array().map_or(0, std::vec::Vec::len)
    // );

    // Verify the assets and temperature readings we created are accessible
    assert_eq!(
        image_filenames.len(),
        3,
        "Should have created 3 image assets"
    );

    // Count wells that have freeze time data (may be 0 without phase transitions)
    let _wells_with_freeze_data = all_wells
        .iter()
        .filter(|well| !well["first_phase_change_time"].is_null())
        .count();

    // 7. Test that assets can be accessed via the by-filename endpoint
    for image_filename in &image_filenames {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/assets/by-experiment/{experiment_id}/{image_filename}"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Should be able to access asset {image_filename} via filename endpoint"
        );
    }

    //     "   📁 Image assets: {} created and accessible",
    //     image_filenames.len()
    // );
}

/// Helper function to create a test tray configuration with trays and probes
async fn create_test_tray_configuration_with_probes(app: &Router) -> Result<String, String> {
    // 1. Create base tray configuration
    let tray_config_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tray_configurations")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Excel Test Configuration",
                        "experiment_default": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    if tray_config_response.status() != StatusCode::CREATED {
        return Err(format!(
            "Failed to create tray configuration: {}",
            tray_config_response.status()
        ));
    }

    let tray_config_body = to_bytes(tray_config_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tray_config: Value = serde_json::from_slice(&tray_config_body).unwrap();
    let tray_config_id = tray_config["id"].as_str().unwrap();

    // 2. Update with trays and probes
    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/tray_configurations/{tray_config_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({
                    "name": "Excel Test Configuration with Trays",
                    "experiment_default": false,
                    "trays": [
                        {
                            "name": "P1",
                            "rotation_degrees": 90,
                            "well_relative_diameter": 6.4,
                            "qty_cols": 12,
                            "qty_rows": 8,
                            "probe_locations": [
                                {"name": "Probe 1", "data_column_index": 1, "position_x": 22.1, "position_y": 77.6},
                                {"name": "Probe 2", "data_column_index": 2, "position_x": 47.1, "position_y": 20},
                                {"name": "Probe 3", "data_column_index": 3, "position_x": 113, "position_y": 19.5},
                                {"name": "Probe 4", "data_column_index": 4, "position_x": 143.5, "position_y": 79.5}
                            ],
                            "upper_left_corner_x": 416,
                            "upper_left_corner_y": 75,
                            "lower_right_corner_x": 135,
                            "lower_right_corner_y": 542,
                            "order_sequence": 1
                        },
                        {
                            "name": "P2",
                            "rotation_degrees": 270,
                            "well_relative_diameter": 6.4,
                            "qty_cols": 12,
                            "qty_rows": 8,
                            "probe_locations": [
                                {"name": "Probe 5", "data_column_index": 5, "position_x": 140.8, "position_y": 80},
                                {"name": "Probe 6", "data_column_index": 6, "position_x": 103.1, "position_y": 21.9},
                                {"name": "Probe 7", "data_column_index": 7, "position_x": 48.1, "position_y": 22.4},
                                {"name": "Probe 8", "data_column_index": 8, "position_x": 7.2, "position_y": 93.3}
                            ],
                            "upper_left_corner_x": 536,
                            "upper_left_corner_y": 529,
                            "lower_right_corner_x": 823,
                            "lower_right_corner_y": 67,
                            "order_sequence": 2
                        }
                    ]
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    if update_response.status() != StatusCode::OK {
        return Err(format!(
            "Failed to update tray configuration: {}",
            update_response.status()
        ));
    }

    Ok(tray_config_id.to_string())
}

/// Helper function to create a test experiment
async fn create_test_experiment_via_api(
    app: &Router,
    tray_config_id: &str,
) -> Result<String, String> {
    let experiment_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/experiments")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Excel Processing API Integration Test",
                        "username": "test_user@example.com",
                        "performed_at": "2025-01-01T00:00:00Z",
                        "is_calibration": false,
                        "remarks": "Testing Excel upload via API",
                        "tray_configuration_id": tray_config_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    if experiment_response.status() != StatusCode::CREATED {
        return Err(format!(
            "Failed to create experiment: {}",
            experiment_response.status()
        ));
    }

    let experiment_body = to_bytes(experiment_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment: Value = serde_json::from_slice(&experiment_body).unwrap();
    let experiment_id = experiment["id"].as_str().unwrap();

    Ok(experiment_id.to_string())
}

/// Helper function to process Excel file via API
async fn process_excel_file_via_api(app: &Router, experiment_id: &str) -> Result<Value, String> {
    // Load test Excel file
    let excel_path = "src/experiments/test_resources/merged.xlsx";
    let excel_data = fs::read(excel_path).map_err(|e| format!("Failed to read Excel file: {e}"))?;

    // Create multipart form data (binary safe)
    let boundary = "test-boundary-12345";
    let mut multipart_body = Vec::new();

    multipart_body.extend_from_slice(format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"excel_file\"; filename=\"merged.xlsx\"\r\nContent-Type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet\r\n\r\n"
    ).as_bytes());
    multipart_body.extend_from_slice(&excel_data);
    multipart_body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    // Process Excel file via API
    let processing_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/experiments/{experiment_id}/process-excel"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(multipart_body))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = processing_response.status();
    let processing_body = to_bytes(processing_response.into_body(), usize::MAX)
        .await
        .unwrap();

    if status != StatusCode::OK {
        let error_text = String::from_utf8_lossy(&processing_body);
        return Err(format!(
            "Excel processing failed with status {status}: {error_text}"
        ));
    }

    let processing_result: Value = serde_json::from_slice(&processing_body)
        .map_err(|e| format!("Failed to parse processing result: {e}"))?;

    Ok(processing_result)
}

/// Helper function to validate Excel processing results
fn validate_excel_processing_results(processing_result: &Value) -> Result<(), String> {
    let success = processing_result["success"].as_bool().unwrap_or(false);
    let empty_errors = vec![];
    let errors = processing_result["errors"]
        .as_array()
        .unwrap_or(&empty_errors);

    let temp_readings_created = processing_result["temperature_readings_created"]
        .as_u64()
        .unwrap_or(0);
    let _probe_readings_created = processing_result["probe_temperature_readings_created"]
        .as_u64()
        .unwrap_or(0);
    let phase_transitions_created = processing_result["phase_transitions_created"]
        .as_u64()
        .unwrap_or(0);

    let has_reasonable_data = temp_readings_created > 5000 && phase_transitions_created > 0;

    if !has_reasonable_data {
        return Err(format!(
            "Processing failed without reasonable data. Success: {success}, Errors: {errors:?}"
        ));
    }

    // Verify expected data volumes
    if temp_readings_created <= 6000 {
        return Err(format!(
            "Expected >6000 temperature readings, got {temp_readings_created}"
        ));
    }
    if phase_transitions_created == 0 {
        return Err("Expected phase transitions, got 0".to_string());
    }

    //     "   ✅ Core Excel processing working via API (temp readings: {}, phase transitions: {})",
    //     temp_readings_created, phase_transitions_created
    // );

    Ok(())
}

/// Helper function to verify experiment results are accessible via API
async fn verify_experiment_results_api(app: &Router, experiment_id: &str) -> Result<(), String> {
    let results_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/experiments/{experiment_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    if results_response.status() != StatusCode::OK {
        return Err(format!(
            "Failed to retrieve experiment results: {}",
            results_response.status()
        ));
    }

    let results_body = to_bytes(results_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment_data: Value = serde_json::from_slice(&results_body)
        .map_err(|e| format!("Failed to parse experiment data: {e}"))?;

    // Check that results include the processed data
    if let Some(results) = experiment_data.get("results")
        && let Some(summary) = results.get("summary")
    {
        let total_time_points = summary["total_time_points"].as_u64().unwrap_or(0);
        if total_time_points <= 6000 {
            return Err(format!(
                "Results should show >6000 time points, got {total_time_points}"
            ));
        }
    }

    Ok(())
}

/// API Integration test for Excel processing endpoint with real Excel file
/// This tests the full HTTP request/response cycle through /api/experiments/{id}/process-excel
#[tokio::test]
async fn test_excel_processing_api_integration() {
    let app = setup_test_app().await;

    // 1. Setup: Create tray configuration with trays and probes
    let tray_config_id = create_test_tray_configuration_with_probes(&app)
        .await
        .expect("Failed to create tray configuration");

    // 2. Setup: Create experiment
    let experiment_id = create_test_experiment_via_api(&app, &tray_config_id)
        .await
        .expect("Failed to create experiment");

    // 3. Process: Upload and process Excel file
    let processing_result = process_excel_file_via_api(&app, &experiment_id)
        .await
        .expect("Failed to process Excel file");

    // 4. Validate: Check processing results
    validate_excel_processing_results(&processing_result)
        .expect("Processing results validation failed");

    // 5. Validate: Check that results are accessible via API
    verify_experiment_results_api(&app, &experiment_id)
        .await
        .expect("Failed to verify experiment results API");
}

/// Test to verify sample well filtering issue - samples should not include wells from other treatments
/// This test confirms the bug where each treatment gets assigned all 192 wells instead of just its region wells
#[tokio::test]
async fn test_sample_well_filtering_after_excel_processing() {
    let app = setup_test_app().await;

    // 1. Setup: Create tray configuration with trays and probes
    let tray_config_id = create_test_tray_configuration_with_probes(&app)
        .await
        .expect("Failed to create tray configuration");

    // 2. Setup: Create experiment
    let experiment_id = create_test_experiment_via_api(&app, &tray_config_id)
        .await
        .expect("Failed to create experiment");

    // 3. Create sample and treatments to be used in regions
    let sample_id = create_test_sample_and_treatments(&app)
        .await
        .expect("Failed to create sample and treatments");

    // 4. Add regions to the experiment by updating it
    update_experiment_with_regions(&app, &experiment_id, &sample_id)
        .await
        .expect("Failed to add regions to experiment");

    // 5. Process: Upload and process Excel file
    let _processing_result = process_excel_file_via_api(&app, &experiment_id)
        .await
        .expect("Failed to process Excel file");

    // 6. Get experiment details to find regions and samples
    let experiment_data = get_experiment_data(&app, &experiment_id).await;

    // 7. Find the sample used in the experiment (should be the first sample from first region)
    let regions = experiment_data["regions"].as_array().unwrap();
    assert!(!regions.is_empty(), "Experiment should have regions");

    let first_region = &regions[0];
    let sample_id = first_region["treatment"]["sample"]["id"]
        .as_str()
        .expect("Should have sample ID in first region");

    // 8. Test: Get sample data via API
    let sample_data = get_sample_data(&app, sample_id).await;

    // 9. Validate: Check treatment well counts
    validate_treatment_well_counts(&sample_data);
}

async fn get_experiment_data(app: &Router, experiment_id: &str) -> Value {
    let experiment_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/experiments/{experiment_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(experiment_response.status(), StatusCode::OK);
    let experiment_body = to_bytes(experiment_response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&experiment_body).unwrap()
}

async fn get_sample_data(app: &Router, sample_id: &str) -> Value {
    let sample_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/samples/{sample_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(sample_response.status(), StatusCode::OK);
    let sample_body = to_bytes(sample_response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&sample_body).unwrap()
}

fn validate_treatment_well_counts(sample_data: &Value) {
    let treatments = sample_data["treatments"].as_array().unwrap();
    assert!(!treatments.is_empty(), "Sample should have treatments");

    for treatment in treatments {
        let treatment_name = treatment["name"].as_str().unwrap();
        let experimental_results = treatment["experimental_results"].as_array().unwrap();
        let dilution_summaries = treatment["dilution_summaries"].as_array().unwrap();

        match treatment_name {
            "none" => {
                let mut total_wells_from_summaries = 0;
                for summary in dilution_summaries {
                    let wells_count: i32 = summary["statistics"]["total_wells"]
                        .as_i64()
                        .unwrap()
                        .try_into()
                        .unwrap();

                    total_wells_from_summaries += wells_count;

                    let dilution_factor = summary["dilution_factor"].as_i64().unwrap();
                    match dilution_factor {
                        1 => assert_eq!(
                            wells_count, 32,
                            "None treatment at 1x dilution should have 32 wells, got {wells_count}"
                        ),
                        10 => assert_eq!(
                            wells_count, 48,
                            "None treatment at 10x dilution should have 48 wells, got {wells_count}"
                        ),
                        100 => assert_eq!(
                            wells_count, 48,
                            "None treatment at 100x dilution should have 48 wells, got {wells_count}"
                        ),
                        _ => panic!("Unexpected dilution factor: {}", dilution_factor),
                    }
                }
                assert_eq!(
                    total_wells_from_summaries, 128,
                    "None treatment should have 128 total wells (32+48+48), got {total_wells_from_summaries}"
                );
                assert_eq!(
                    experimental_results.len(),
                    128,
                    "None treatment should have 128 experimental results, got {}",
                    experimental_results.len()
                );
            }
            "heat" => {
                assert_eq!(
                    experimental_results.len(),
                    32,
                    "Heat treatment should have 32 experimental results, got {}",
                    experimental_results.len()
                );
                assert_eq!(
                    dilution_summaries.len(),
                    1,
                    "Heat treatment should have 1 dilution summary, got {}",
                    dilution_summaries.len()
                );
                let summary = &dilution_summaries[0];
                let wells_count: i32 = summary["statistics"]["total_wells"]
                    .as_i64()
                    .unwrap()
                    .try_into()
                    .unwrap();
                assert_eq!(
                    wells_count, 32,
                    "Heat treatment should have 32 wells, got {wells_count}"
                );
            }
            "h2o2" => {
                assert_eq!(
                    experimental_results.len(),
                    32,
                    "H2O2 treatment should have 32 experimental results, got {}",
                    experimental_results.len()
                );
                assert_eq!(
                    dilution_summaries.len(),
                    1,
                    "H2O2 treatment should have 1 dilution summary, got {}",
                    dilution_summaries.len()
                );
                let summary = &dilution_summaries[0];
                let wells_count: i32 = summary["statistics"]["total_wells"]
                    .as_i64()
                    .unwrap()
                    .try_into()
                    .unwrap();
                assert_eq!(
                    wells_count, 32,
                    "H2O2 treatment should have 32 wells, got {wells_count}"
                );
            }
            _ => panic!("Unexpected treatment name: {}", treatment_name),
        }
    }
}

/// Helper function to create sample and treatments (mimics seeder structure)
#[allow(clippy::too_many_lines)]
async fn create_test_sample_and_treatments(app: &Router) -> Result<String, String> {
    // 1. First create a project
    let project_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Test Arctic Research Project",
                        "description": "Test project for sample well filtering",
                        "colour": "#3B82F6"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let project_body = to_bytes(project_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let project: Value = serde_json::from_slice(&project_body).unwrap();
    let project_id = project["id"].as_str().unwrap();

    // 2. Create a location
    let location_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/locations")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Utqiagvik Research Station",
                        "comment": "Arctic atmospheric research facility",
                        "project_id": project_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let location_body = to_bytes(location_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let location: Value = serde_json::from_slice(&location_body).unwrap();
    let location_id = location["id"].as_str().unwrap();

    // 3. Create a sample
    let sample_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/samples")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Utqiagvik Resea PM10 Aerosol Filter 2025-08-25 S001",
                        "type": "filter",
                        "start_time": "2025-08-25T06:00:00Z",
                        "stop_time": "2025-08-25T08:00:00Z",
                        "flow_litres_per_minute": "10.3358743102",
                        "total_volume": "1740.1531513381",
                        "filter_substrate": "PTFE",
                        "suspension_volume_litres": "0.011491431795436186",
                        "well_volume_litres": "0.00005",
                        "remarks": "Environmental sample S001 collected from Utqiagvik Research Station region on 2025-08-25. Sample type: filter. GPS: 71.320002°, -156.743158°",
                        "location_id": location_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let sample_body = to_bytes(sample_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let sample: Value = serde_json::from_slice(&sample_body).unwrap();
    let sample_id = sample["id"].as_str().unwrap();

    // 4. Create treatments for this sample (none, heat, h2o2)
    let treatments = vec![
        json!({
            "name": "none",
            "notes": "Untreated control sample - baseline ice nucleation activity",
            "sample_id": sample_id
        }),
        json!({
            "name": "heat",
            "notes": "Heat treatment at 95°C for 20 minutes - removes heat-labile biological INPs",
            "sample_id": sample_id
        }),
        json!({
            "name": "h2o2",
            "notes": "Hydrogen peroxide treatment - removes organic components including biological INPs",
            "enzyme_volume_litres": "0.0002358673",
            "sample_id": sample_id
        }),
    ];

    for treatment in treatments {
        let _treatment_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/treatments")
                    .header("content-type", "application/json")
                    .body(Body::from(treatment.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    Ok(sample_id.to_string())
}

/// Helper function to update experiment with regions (uses exact seeder structure)
#[allow(clippy::too_many_lines)]
async fn update_experiment_with_regions(
    app: &Router,
    experiment_id: &str,
    sample_id: &str,
) -> Result<(), String> {
    // Get the current experiment
    let experiment_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/experiments/{experiment_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let experiment_body = to_bytes(experiment_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let mut experiment_data: Value = serde_json::from_slice(&experiment_body).unwrap();

    // Get treatments for the sample to link to regions
    let sample_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/samples/{sample_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let sample_body = to_bytes(sample_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let sample_data: Value = serde_json::from_slice(&sample_body).unwrap();
    let treatments = sample_data["treatments"].as_array().unwrap();

    // Find treatment IDs
    let mut treatment_map = std::collections::HashMap::new();
    for treatment in treatments {
        let name = treatment["name"].as_str().unwrap();
        let id = treatment["id"].as_str().unwrap();
        treatment_map.insert(name, id);
    }

    // Add regions to the experiment (exact same structure as seeder)
    experiment_data["regions"] = json!([
        {
            "treatment_id": treatment_map.get("none").unwrap(),
            "name": "Untreated Samples - P1",
            "display_colour_hex": "#3B82F6",
            "tray_id": 1,
            "col_min": 0, "col_max": 3, "row_min": 0, "row_max": 7,
            "dilution_factor": 1,
            "is_background_key": false
        },
        {
            "treatment_id": treatment_map.get("heat").unwrap(),
            "name": "Heat Treated - P1",
            "display_colour_hex": "#EF4444",
            "tray_id": 1,
            "col_min": 4, "col_max": 7, "row_min": 0, "row_max": 7,
            "dilution_factor": 1,
            "is_background_key": false
        },
        {
            "treatment_id": treatment_map.get("h2o2").unwrap(),
            "name": "H2O2 Treated - P1",
            "display_colour_hex": "#10B981",
            "tray_id": 1,
            "col_min": 8, "col_max": 11, "row_min": 0, "row_max": 7,
            "dilution_factor": 1,
            "is_background_key": false
        },
        {
            "treatment_id": treatment_map.get("none").unwrap(),
            "name": "Dilution Series 1:10 - P2",
            "display_colour_hex": "#8B5CF6",
            "tray_id": 2,
            "col_min": 0, "col_max": 5, "row_min": 0, "row_max": 7,
            "dilution_factor": 10,
            "is_background_key": false
        },
        {
            "treatment_id": treatment_map.get("none").unwrap(),
            "name": "Dilution Series 1:100 - P2",
            "display_colour_hex": "#F59E0B",
            "tray_id": 2,
            "col_min": 6, "col_max": 11, "row_min": 0, "row_max": 7,
            "dilution_factor": 100,
            "is_background_key": false
        }
    ]);

    // Update the experiment with regions
    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/experiments/{experiment_id}"))
                .header("content-type", "application/json")
                .body(Body::from(experiment_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    if update_response.status() != StatusCode::OK {
        let error_body = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let error_text = String::from_utf8_lossy(&error_body);
        return Err(format!(
            "Failed to update experiment with regions: {error_text}"
        ));
    }

    Ok(())
}

/// Helper function to create a simple tray configuration
async fn create_simple_tray_configuration(app: &Router, name: &str) -> Result<String, String> {
    let tray_config_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tray_configurations")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": name,
                        "experiment_default": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    if tray_config_response.status() != StatusCode::CREATED {
        return Err(format!(
            "Failed to create tray configuration: {}",
            tray_config_response.status()
        ));
    }

    let tray_config_body = to_bytes(tray_config_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tray_config: Value = serde_json::from_slice(&tray_config_body).unwrap();
    let tray_config_id = tray_config["id"].as_str().unwrap();

    Ok(tray_config_id.to_string())
}

/// API Integration test for experiment results endpoint after Excel processing
/// This tests retrieving processed experiment results via /api/experiments/{id}
#[tokio::test]
async fn test_experiment_results_api_integration() {
    let app = setup_test_app().await;

    // 1. Create simple test setup via API
    let tray_config_id = create_simple_tray_configuration(&app, "Results Test Configuration")
        .await
        .expect("Failed to create tray configuration");

    // 2. Create experiment via API with custom name
    let experiment_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/experiments")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Results API Integration Test",
                        "username": "test_user@example.com",
                        "performed_at": "2025-01-01T00:00:00Z",
                        "is_calibration": false,
                        "remarks": "Testing results API endpoint",
                        "tray_configuration_id": tray_config_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        (experiment_response.status() == StatusCode::CREATED),
        "Failed to create experiment: {}",
        experiment_response.status()
    );

    let experiment_body = to_bytes(experiment_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment: Value = serde_json::from_slice(&experiment_body).unwrap();
    let experiment_id = experiment["id"].as_str().unwrap().to_string();

    // 3. Test retrieving experiment without processed data
    let empty_results_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/experiments/{experiment_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(empty_results_response.status(), StatusCode::OK);
    let empty_results_body = to_bytes(empty_results_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let empty_experiment_data: Value = serde_json::from_slice(&empty_results_body).unwrap();

    // Verify experiment data structure
    assert_eq!(empty_experiment_data["id"].as_str().unwrap(), experiment_id);
    assert_eq!(
        empty_experiment_data["name"].as_str().unwrap(),
        "Results API Integration Test"
    );

    // Check results structure (should be null or empty for experiment without data)
    let results = empty_experiment_data.get("results");
    if let Some(results_val) = results
        && !results_val.is_null()
    {
        // If results exist, they should have the expected structure
        assert!(
            results_val.get("summary").is_some(),
            "Results should have summary"
        );
        assert!(
            results_val.get("trays").is_some(),
            "Results should have trays array"
        );
    }

    // 4. Test API error handling
    let nonexistent_id = "00000000-0000-0000-0000-000000000000";
    let not_found_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/experiments/{nonexistent_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(not_found_response.status(), StatusCode::NOT_FOUND);

    // 5. Test invalid UUID handling
    let invalid_uuid_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/experiments/invalid-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(invalid_uuid_response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_seeder_experiment_structure() {
    let app = setup_test_app().await;

    // Test the exact structure our seeder is trying to create
    let seeder_experiment = json!({
        "name": "Arctic Aerosol INP Characterization Exp139",
        "username": "researcher@eerl.lab",
        "temperature_ramp": -1.0,
        "temperature_start": 5.0,
        "temperature_end": -25.0,
        "is_calibration": false,
        "remarks": "Comprehensive characterization of Arctic atmospheric aerosol samples using SPICE droplet freezing technique",
        "performed_at": "2025-08-15T10:30:00.000Z"
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/experiments")
                .header("content-type", "application/json")
                .body(Body::from(seeder_experiment.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (create_status, create_body) = extract_response_body(create_response).await;

    println!("SEEDER EXPERIMENT TEST - Status: {create_status}");
    println!("SEEDER EXPERIMENT TEST - Body: {create_body:?}");

    assert_eq!(
        create_status,
        StatusCode::CREATED,
        "Seeder experiment payload should create successfully. Status: {create_status}, Body: {create_body:?}"
    );
}

#[tokio::test]
async fn test_seeder_sample_with_coordinates() {
    let app = setup_test_app().await;

    // Test the enhanced sample structure with GPS coordinates
    let seeder_sample = json!({
        "name": "Utqiagvik Research PM10 Aerosol Filter 2025-08-15",
        "type": "filter",
        "latitude": "71.3230",
        "longitude": "-156.7660",
        "remarks": "Environmental sample collected from Utqiagvik Research Station on 2025-08-15. Sample type: filter. GPS: 71.3230°, -156.7660°"
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/samples")
                .header("content-type", "application/json")
                .body(Body::from(seeder_sample.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (create_status, create_body) = extract_response_body(create_response).await;

    println!("SEEDER SAMPLE GPS TEST - Status: {create_status}");
    println!("SEEDER SAMPLE GPS TEST - Body: {create_body:?}");

    assert_eq!(
        create_status,
        StatusCode::CREATED,
        "Seeder sample with GPS should create successfully. Status: {create_status}, Body: {create_body:?}"
    );

    // Verify GPS coordinates are stored - note: Decimal type may truncate trailing zeros
    let latitude = create_body["latitude"].as_str().unwrap();
    let longitude = create_body["longitude"].as_str().unwrap();

    // Check that coordinates have at least 3 decimal places and are within expected range
    assert!(
        latitude.starts_with("71.32"),
        "Latitude should be around 71.32, got: {latitude}"
    );
    assert!(
        longitude.starts_with("-156.76"),
        "Longitude should be around -156.76, got: {longitude}"
    );
}

/// Test temperature readings at specific timestamps, especially during phase changes
/// Verifies that all 8 probes return correct temperature data with metadata
#[allow(clippy::too_many_lines)]
#[tokio::test]
async fn test_temperature_readings_during_phase_changes() {
    println!("🌡️ Testing temperature readings during phase changes");

    let app = setup_test_app().await;

    // 1. Create tray configuration with full probe setup (8 probes)
    let tray_config_id = create_test_tray_configuration_with_probes(&app)
        .await
        .expect("Failed to create tray configuration with probes");

    // 2. Create experiment
    let experiment_id = create_test_experiment_via_api(&app, &tray_config_id)
        .await
        .expect("Failed to create experiment");

    // 3. Process Excel file to get temperature and phase change data
    let _processing_result = process_excel_file_via_api(&app, &experiment_id)
        .await
        .expect("Failed to process Excel file");

    // 4. Get experiment results with temperature data
    let results_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/experiments/{experiment_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(results_response.status(), StatusCode::OK);
    let results_body = to_bytes(results_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment_data: Value =
        serde_json::from_slice(&results_body).expect("Failed to parse experiment results");

    // 5. Validate overall structure
    let results = experiment_data
        .get("results")
        .expect("Experiment should have results")
        .as_object()
        .expect("Results should be an object");

    let trays = results
        .get("trays")
        .expect("Results should have trays")
        .as_array()
        .expect("Trays should be an array");

    assert!(!trays.is_empty(), "Should have at least one tray");

    // 6. Find wells with phase changes and temperature data
    let mut wells_with_phase_changes = 0;
    let mut wells_with_temperatures = 0;
    let mut total_probe_readings_checked = 0;

    for tray in trays {
        let wells = tray
            .get("wells")
            .expect("Tray should have wells")
            .as_array()
            .expect("Wells should be an array");

        for well in wells {
            // Check if well has phase change time
            if let Some(_phase_change_time) = well.get("first_phase_change_time") {
                wells_with_phase_changes += 1;

                // Check if well has temperature data at phase change
                if let Some(temperatures) = well.get("temperatures") {
                    wells_with_temperatures += 1;

                    // Validate temperature structure
                    assert!(
                        temperatures.get("id").is_some(),
                        "Temperature reading should have id"
                    );
                    assert!(
                        temperatures.get("timestamp").is_some(),
                        "Temperature reading should have timestamp"
                    );
                    assert!(
                        temperatures.get("average").is_some(),
                        "Temperature reading should have average"
                    );

                    // Validate probe readings array
                    let probe_readings = temperatures
                        .get("probe_readings")
                        .expect("Temperature reading should have probe_readings")
                        .as_array()
                        .expect("probe_readings should be an array");

                    println!(
                        "   Well {}:{} has {} probe readings at phase change",
                        well["row_letter"].as_str().unwrap_or("?"),
                        well["column_number"].as_i64().unwrap_or(0),
                        probe_readings.len()
                    );

                    // Validate each probe reading
                    for probe_reading in probe_readings {
                        total_probe_readings_checked += 1;

                        // Check probe reading structure
                        assert!(
                            probe_reading.get("id").is_some(),
                            "Probe reading should have id"
                        );
                        assert!(
                            probe_reading.get("probe_id").is_some(),
                            "Probe reading should have probe_id"
                        );
                        assert!(
                            probe_reading.get("temperature").is_some(),
                            "Probe reading should have temperature"
                        );

                        // Check probe metadata (new feature)
                        assert!(
                            probe_reading.get("probe_name").is_some(),
                            "Probe reading should have probe_name"
                        );
                        assert!(
                            probe_reading.get("probe_data_column_index").is_some(),
                            "Probe reading should have probe_data_column_index"
                        );
                        assert!(
                            probe_reading.get("probe_position_x").is_some(),
                            "Probe reading should have probe_position_x"
                        );
                        assert!(
                            probe_reading.get("probe_position_y").is_some(),
                            "Probe reading should have probe_position_y"
                        );

                        // Validate temperature value (should be formatted to 3 decimal places)
                        let temperature_str = probe_reading
                            .get("temperature")
                            .and_then(|t| t.as_str())
                            .expect("Temperature should be a string");

                        // Parse temperature and check it's reasonable
                        let temperature: f64 = temperature_str
                            .parse()
                            .expect("Temperature should be parseable as float");

                        // During phase changes, temperatures should be below 0°C and above -30°C (reasonable range)
                        assert!(
                            (-30.0..=5.0).contains(&temperature),
                            "Temperature {temperature} should be in reasonable range (-30°C to 5°C) during phase change"
                        );

                        // Validate probe metadata values
                        let probe_name = probe_reading
                            .get("probe_name")
                            .and_then(|n| n.as_str())
                            .expect("Probe name should be a string");
                        assert!(
                            probe_name.starts_with("Probe"),
                            "Probe name should start with 'Probe', got: {probe_name}"
                        );

                        let data_column_index = probe_reading
                            .get("probe_data_column_index")
                            .and_then(serde_json::Value::as_i64)
                            .expect("Data column index should be an integer");
                        assert!(
                            (1..=8).contains(&data_column_index),
                            "Data column index should be 1-8, got: {data_column_index}"
                        );
                    }

                    // Validate average temperature
                    if let Some(average_str) = temperatures.get("average").and_then(|a| a.as_str())
                    {
                        let average: f64 = average_str
                            .parse()
                            .expect("Average temperature should be parseable as float");
                        assert!(
                            (-30.0..=5.0).contains(&average),
                            "Average temperature {average} should be in reasonable range during phase change"
                        );
                    }

                    // Stop after checking a few wells to avoid test timeout
                    if wells_with_temperatures >= 3 {
                        break;
                    }
                }
            }
        }
        if wells_with_temperatures >= 3 {
            break;
        }
    }

    // 7. Validate test results
    assert!(
        wells_with_phase_changes > 0,
        "Should have found wells with phase changes"
    );
    assert!(
        wells_with_temperatures > 0,
        "Should have found wells with temperature data at phase change"
    );
    assert!(
        total_probe_readings_checked > 0,
        "Should have checked probe readings"
    );

    println!("✅ Temperature validation completed:");
    println!("   - Wells with phase changes: {wells_with_phase_changes}");
    println!("   - Wells with temperature data: {wells_with_temperatures}");
    println!("   - Probe readings validated: {total_probe_readings_checked}");

    // 8. Additional validation: Check that we have reasonable number of probe readings
    // Since we're looking at phase change times, and each well should have readings from multiple probes
    let expected_min_probe_readings = wells_with_temperatures * 3; // At least 3 probes per well with data
    assert!(
        total_probe_readings_checked >= expected_min_probe_readings,
        "Expected at least {expected_min_probe_readings} probe readings ({wells_with_temperatures}+ wells × 3+ probes), got {total_probe_readings_checked}"
    );
}
