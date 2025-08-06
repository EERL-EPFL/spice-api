use crate::config::test_helpers::setup_test_app;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn extract_response_body(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Error reading response body");
    let body: Value = serde_json::from_slice(&bytes).expect("Error parsing response JSON");
    (status, body)
}

#[tokio::test]
async fn test_create_location() {
    let app = setup_test_app().await;
    let location_data = json!({
        "name": "Test Location",
        "comment": "A test location for experiments"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/locations")
                .header("Content-Type", "application/json")
                .body(Body::from(location_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, body) = extract_response_body(response).await;
    
    assert_eq!(status, StatusCode::CREATED);
    assert!(body["id"].is_string());
    assert_eq!(body["name"], "Test Location");
    assert_eq!(body["comment"], "A test location for experiments");
}