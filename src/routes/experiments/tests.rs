
use crate::config::test_helpers::setup_test_app;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

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
                .uri(&format!("/api/experiments/{}", experiment_id))
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

    println!(
        "Experiment response: {}",
        serde_json::to_string_pretty(&experiment_with_results).unwrap()
    );

    // Check that results_summary is included
    assert!(
        experiment_with_results["results_summary"].is_object(),
        "Should have results_summary object"
    );

    let results_summary = &experiment_with_results["results_summary"];

    // Check required fields exist
    assert!(
        results_summary["total_wells"].is_number(),
        "Should have total_wells"
    );
    assert!(
        results_summary["wells_with_data"].is_number(),
        "Should have wells_with_data"
    );
    assert!(
        results_summary["wells_frozen"].is_number(),
        "Should have wells_frozen"
    );
    assert!(
        results_summary["wells_liquid"].is_number(),
        "Should have wells_liquid"
    );
    assert!(
        results_summary["total_time_points"].is_number(),
        "Should have total_time_points"
    );
    assert!(
        results_summary["well_summaries"].is_array(),
        "Should have well_summaries array"
    );

    // For a new experiment with no data, we expect 0 values
    assert_eq!(
        results_summary["total_wells"], 0,
        "New experiment should have 0 wells"
    );
    assert_eq!(
        results_summary["wells_with_data"], 0,
        "New experiment should have 0 wells with data"
    );
    assert_eq!(
        results_summary["total_time_points"], 0,
        "New experiment should have 0 time points"
    );
}

#[tokio::test]
async fn test_experiment_with_phase_transitions_data() {
    use crate::config::test_helpers::setup_test_db;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set};

    use spice_entity::{
        experiments, regions, samples, temperature_readings, tray_configuration_assignments,
        tray_configurations, trays, treatments, well_phase_transitions, wells,
    };
    use uuid::Uuid;

    let db = setup_test_db().await;

    // Create tray
    let tray_id = Uuid::new_v4();
    let tray = trays::ActiveModel {
        id: Set(tray_id),
        name: Set(Some("Test Tray".to_string())),
        qty_x_axis: Set(Some(2)),
        qty_y_axis: Set(Some(2)),
        well_relative_diameter: Set(None),
        last_updated: Set(chrono::Utc::now().into()),
        created_at: Set(chrono::Utc::now().into()),
    };
    tray.insert(&db).await.unwrap();

    // Create tray configuration
    let config_id = Uuid::new_v4();
    let config = tray_configurations::ActiveModel {
        id: Set(config_id),
        name: Set(Some("Test Config".to_string())),
        experiment_default: Set(false),
        created_at: Set(chrono::Utc::now().into()),
        last_updated: Set(chrono::Utc::now().into()),
    };
    config.insert(&db).await.unwrap();

    // Create tray configuration assignment
    let assignment = tray_configuration_assignments::ActiveModel {
        tray_id: Set(tray_id),
        tray_configuration_id: Set(config_id),
        order_sequence: Set(1),
        rotation_degrees: Set(0),
        created_at: Set(chrono::Utc::now().into()),
        last_updated: Set(chrono::Utc::now().into()),
    };
    assignment.insert(&db).await.unwrap();

    // Create experiment
    let experiment_id = Uuid::new_v4();
    let experiment = experiments::ActiveModel {
        id: Set(experiment_id),
        name: Set("Test Experiment".to_string()),
        username: Set(Some("test@example.com".to_string())),
        tray_configuration_id: Set(Some(config_id)),
        performed_at: Set(Some(chrono::Utc::now().into())),
        temperature_ramp: Set(Some(rust_decimal::Decimal::new(-1, 0))),
        temperature_start: Set(Some(rust_decimal::Decimal::new(5, 0))),
        temperature_end: Set(Some(rust_decimal::Decimal::new(-25, 0))),
        is_calibration: Set(false),
        remarks: Set(Some("Test experiment".to_string())),
        created_at: Set(chrono::Utc::now().into()),
        last_updated: Set(chrono::Utc::now().into()),
    };
    experiment.insert(&db).await.unwrap();

    // Create wells
    let well_1_id = Uuid::new_v4();
    let well_1 = wells::ActiveModel {
        id: Set(well_1_id),
        tray_id: Set(tray_id),
        row_number: Set(1),
        column_number: Set(1),
        created_at: Set(chrono::Utc::now().into()),
        last_updated: Set(chrono::Utc::now().into()),
    };
    well_1.insert(&db).await.unwrap();

    // Create temperature reading
    let temp_reading_id = Uuid::new_v4();
    let temp_reading = temperature_readings::ActiveModel {
        id: Set(temp_reading_id),
        experiment_id: Set(experiment_id),
        timestamp: Set(chrono::Utc::now().into()),
        image_filename: Set(Some("test.jpg".to_string())),
        probe_1: Set(Some(rust_decimal::Decimal::new(250, 1))), // 25.0
        probe_2: Set(Some(rust_decimal::Decimal::new(240, 1))), // 24.0
        probe_3: Set(Some(rust_decimal::Decimal::new(260, 1))), // 26.0
        probe_4: Set(None),
        probe_5: Set(None),
        probe_6: Set(None),
        probe_7: Set(None),
        probe_8: Set(None),
        created_at: Set(chrono::Utc::now().into()),
    };
    temp_reading.insert(&db).await.unwrap();

    // Create phase transition
    let phase_transition = well_phase_transitions::ActiveModel {
        id: Set(Uuid::new_v4()),
        well_id: Set(well_1_id),
        experiment_id: Set(experiment_id),
        temperature_reading_id: Set(temp_reading_id),
        timestamp: Set(chrono::Utc::now().into()),
        previous_state: Set(0), // liquid
        new_state: Set(1),      // frozen
        created_at: Set(chrono::Utc::now().into()),
    };
    phase_transition.insert(&db).await.unwrap();

    // Create sample and treatment
    let sample_id = Uuid::new_v4();
    let sample = samples::ActiveModel {
        id: Set(sample_id),
        name: Set("Test Sample".to_string()),
        start_time: Set(None),
        stop_time: Set(None),
        flow_litres_per_minute: Set(None),
        total_volume: Set(None),
        material_description: Set(None),
        extraction_procedure: Set(None),
        filter_substrate: Set(None),
        suspension_volume_litres: Set(None),
        air_volume_litres: Set(None),
        water_volume_litres: Set(None),
        initial_concentration_gram_l: Set(None),
        well_volume_litres: Set(None),
        remarks: Set(None),
        longitude: Set(None),
        latitude: Set(None),
        location_id: Set(None),
        created_at: Set(chrono::Utc::now().into()),
        last_updated: Set(chrono::Utc::now().into()),
        r#type: Set(spice_entity::sea_orm_active_enums::SampleType::Filter),
    };
    sample.insert(&db).await.unwrap();

    let treatment_id = Uuid::new_v4();
    let treatment = treatments::ActiveModel {
        id: Set(treatment_id),
        notes: Set(Some("Test treatment".to_string())),
        sample_id: Set(Some(sample_id)),
        last_updated: Set(chrono::Utc::now().into()),
        created_at: Set(chrono::Utc::now().into()),
        enzyme_volume_litres: Set(None),
        name: Set(spice_entity::sea_orm_active_enums::TreatmentName::None),
    };
    treatment.insert(&db).await.unwrap();

    // Create region
    let region = regions::ActiveModel {
        id: Set(Uuid::new_v4()),
        experiment_id: Set(experiment_id),
        treatment_id: Set(Some(treatment_id)),
        name: Set(Some("Test Region".to_string())),
        display_colour_hex: Set(Some("#FF0000".to_string())),
        tray_id: Set(Some(1)),
        col_min: Set(Some(1)),
        row_min: Set(Some(1)),
        col_max: Set(Some(1)),
        row_max: Set(Some(1)),
        dilution_factor: Set(Some(100)),
        created_at: Set(chrono::Utc::now().into()),
        last_updated: Set(chrono::Utc::now().into()),
        is_background_key: Set(false),
    };
    region.insert(&db).await.unwrap();

    // Now test the experiment endpoint
    let mut config = crate::config::Config::for_tests();
    config.keycloak_url = String::new();
    let app = crate::routes::build_router(&db, &config);

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri(&format!("/api/experiments/{}", experiment_id))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let experiment_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    println!(
        "Experiment with data response: {}",
        serde_json::to_string_pretty(&experiment_response).unwrap()
    );

    let results_summary = &experiment_response["results_summary"];
    assert!(results_summary.is_object(), "Should have results_summary");

    // Check that we have data
    assert_eq!(results_summary["total_wells"], 1, "Should have 1 well");
    assert_eq!(
        results_summary["wells_with_data"], 1,
        "Should have 1 well with data"
    );
    assert_eq!(
        results_summary["wells_frozen"], 1,
        "Should have 1 frozen well"
    );
    assert_eq!(
        results_summary["wells_liquid"], 0,
        "Should have 0 liquid wells"
    );
    assert_eq!(
        results_summary["total_time_points"], 1,
        "Should have 1 temperature reading"
    );

    let well_summaries = results_summary["well_summaries"].as_array().unwrap();
    assert_eq!(well_summaries.len(), 1, "Should have 1 well summary");

    let well_summary = &well_summaries[0];
    assert_eq!(well_summary["coordinate"], "A1", "Should be coordinate A1");
    assert_eq!(well_summary["final_state"], "frozen", "Should be frozen");
    assert!(
        well_summary["first_phase_change_time"].is_string(),
        "Should have phase change time"
    );
    assert_eq!(
        well_summary["sample_name"], "Test Sample",
        "Should have sample name"
    );
    assert_eq!(
        well_summary["treatment_name"], "None",
        "Should have treatment name"
    );
    assert_eq!(
        well_summary["dilution_factor"], 100,
        "Should have dilution factor"
    );
}
