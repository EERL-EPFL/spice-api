/// Shared test helper functions for creating test objects across the test suite
/// 
/// This module provides standardized builders for creating test entities that follow
/// the object hierarchy: Projects → Locations → Samples → Treatments
/// and TrayConfigurations → Trays → {Probes, Wells}
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

/// Extract response body as JSON for testing
pub async fn extract_response_body(response: axum::response::Response) -> (StatusCode, Value) {
    use axum::body::to_bytes;
    
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"error": "Invalid JSON response"}));
    (status, body)
}

/// Create a test project with default parameters
pub async fn create_test_project(app: &axum::Router) -> Result<(String, Value), String> {
    create_test_project_with_params(
        app,
        &format!("Test Project {}", Uuid::new_v4()),
        "#FF0000",
        Some("Test project created by helper"),
    )
    .await
}

/// Create a test project with customizable parameters
pub async fn create_test_project_with_params(
    app: &axum::Router,
    name: &str,
    colour: &str,
    note: Option<&str>,
) -> Result<(String, Value), String> {
    let mut project_data = json!({
        "name": name,
        "colour": colour
    });

    if let Some(note_text) = note {
        project_data["note"] = json!(note_text);
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header("content-type", "application/json")
                .body(Body::from(project_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, body) = extract_response_body(response).await;

    if status == StatusCode::CREATED {
        let project_id = body["id"].as_str().unwrap().to_string();
        Ok((project_id, body))
    } else {
        Err(format!("Failed to create project: Status {status}, Body: {body}"))
    }
}

/// Create a test location with default parameters
pub async fn create_test_location(
    app: &axum::Router,
    project_id: &str,
) -> Result<(String, Value), String> {
    create_test_location_with_params(
        app,
        &format!("Test Location {}", Uuid::new_v4()),
        Some("Test location created by helper"),
        project_id,
    )
    .await
}

/// Create a test location with customizable parameters
pub async fn create_test_location_with_params(
    app: &axum::Router,
    name: &str,
    comment: Option<&str>,
    project_id: &str,
) -> Result<(String, Value), String> {
    let mut location_data = json!({
        "name": name,
        "project_id": project_id
    });

    if let Some(comment_text) = comment {
        location_data["comment"] = json!(comment_text);
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/locations")
                .header("content-type", "application/json")
                .body(Body::from(location_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, body) = extract_response_body(response).await;

    if status == StatusCode::CREATED {
        let location_id = body["id"].as_str().unwrap().to_string();
        Ok((location_id, body))
    } else {
        Err(format!("Failed to create location: Status {status}, Body: {body}"))
    }
}

/// Create a test sample with default parameters
pub async fn create_test_sample(
    app: &axum::Router,
    location_id: &str,
) -> Result<(String, Value), String> {
    create_test_sample_with_params(
        app,
        &format!("Test Sample {}", Uuid::new_v4()),
        location_id,
        "bulk",
        Some("Test sample created by helper"),
    )
    .await
}

/// Create a test sample with customizable parameters
pub async fn create_test_sample_with_params(
    app: &axum::Router,
    name: &str,
    location_id: &str,
    sample_type: &str,
    comment: Option<&str>,
) -> Result<(String, Value), String> {
    let mut sample_data = json!({
        "name": name,
        "location_id": location_id,
        "sample_type": sample_type
    });

    if let Some(comment_text) = comment {
        sample_data["comment"] = json!(comment_text);
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/samples")
                .header("content-type", "application/json")
                .body(Body::from(sample_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, body) = extract_response_body(response).await;

    if status == StatusCode::CREATED {
        let sample_id = body["id"].as_str().unwrap().to_string();
        Ok((sample_id, body))
    } else {
        Err(format!("Failed to create sample: Status {status}, Body: {body}"))
    }
}

/// Create a test treatment with default parameters
pub async fn create_test_treatment(
    app: &axum::Router,
    sample_id: &str,
) -> Result<(String, Value), String> {
    create_test_treatment_with_params(
        app,
        sample_id,
        "none",
        Some("Test treatment created by helper"),
    )
    .await
}

/// Create a test treatment with customizable parameters
pub async fn create_test_treatment_with_params(
    app: &axum::Router,
    sample_id: &str,
    treatment_name: &str,
    comment: Option<&str>,
) -> Result<(String, Value), String> {
    let mut treatment_data = json!({
        "sample_id": sample_id,
        "name": treatment_name
    });

    if let Some(comment_text) = comment {
        treatment_data["comment"] = json!(comment_text);
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/treatments")
                .header("content-type", "application/json")
                .body(Body::from(treatment_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, body) = extract_response_body(response).await;

    if status == StatusCode::CREATED {
        let treatment_id = body["id"].as_str().unwrap().to_string();
        Ok((treatment_id, body))
    } else {
        Err(format!("Failed to create treatment: Status {status}, Body: {body}"))
    }
}

/// Create a test experiment with default parameters
pub async fn create_test_experiment(
    app: &axum::Router,
) -> Result<(String, Value), String> {
    create_test_experiment_with_params(
        app,
        &format!("Test Experiment {}", Uuid::new_v4()),
        "test@example.com",
        -1.0,
        5.0,
        -25.0,
    )
    .await
}

/// Create a test experiment with customizable parameters
pub async fn create_test_experiment_with_params(
    app: &axum::Router,
    name: &str,
    username: &str,
    temperature_ramp: f64,
    temperature_start: f64,
    temperature_end: f64,
) -> Result<(String, Value), String> {
    let experiment_data = json!({
        "name": name,
        "username": username,
        "performed_at": "2024-06-20T14:30:00Z",
        "temperature_ramp": temperature_ramp,
        "temperature_start": temperature_start,
        "temperature_end": temperature_end,
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

    if status == StatusCode::CREATED || status == StatusCode::OK {
        let experiment_id = body["id"].as_str().unwrap().to_string();
        Ok((experiment_id, body))
    } else {
        Err(format!("Failed to create experiment: Status {status}, Body: {body}"))
    }
}

/// Create a complete object hierarchy: Project → Location → Sample → Treatment
pub async fn create_full_object_hierarchy(
    app: &axum::Router,
) -> Result<FullObjectHierarchy, String> {
    // Create project
    let (project_id, project) = create_test_project(app).await?;
    
    // Create location
    let (location_id, location) = create_test_location(app, &project_id).await?;
    
    // Create sample  
    let (sample_id, sample) = create_test_sample(app, &location_id).await?;
    
    // Create treatment
    let (treatment_id, treatment) = create_test_treatment(app, &sample_id).await?;

    Ok(FullObjectHierarchy {
        project_id,
        project,
        location_id,
        location,
        sample_id,
        sample,
        treatment_id,
        treatment,
    })
}

/// Represents a complete object hierarchy for testing
pub struct FullObjectHierarchy {
    pub project_id: String,
    pub project: Value,
    pub location_id: String,
    pub location: Value,
    pub sample_id: String,
    pub sample: Value,
    pub treatment_id: String,
    pub treatment: Value,
}