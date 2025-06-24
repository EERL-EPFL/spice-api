use super::models::{Sample, SampleCreate, SampleUpdate};
use crate::common::auth::Role;
use crate::common::state::AppState;
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::{CRUDResource, crud_handlers};
use utoipa_axum::{router::OpenApiRouter, routes};

crud_handlers!(Sample, SampleUpdate, SampleCreate);

pub fn router(state: &AppState) -> OpenApiRouter
where
    Sample: CRUDResource,
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
            Sample::RESOURCE_NAME_PLURAL
        );
    }

    mutating_router
}

#[cfg(test)]
mod tests {
    use crate::config::test_helpers::setup_test_app;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn extract_response_body(response: axum::response::Response) -> (StatusCode, Value) {
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        let body: Value = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
            let raw_text = String::from_utf8_lossy(&bytes);
            json!({"error": raw_text})
        });
        (status, body)
    }

    async fn create_test_project_and_location(
        app: &axum::Router,
        test_suffix: &str,
    ) -> (Uuid, Uuid) {
        // Create a test project
        let project_data = json!({
            "name": format!("Test Project {}", test_suffix),
            "note": "Test project for sample tests",
            "colour": "#FF0000"
        });

        let project_response = app
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

        let (project_status, project_body) = extract_response_body(project_response).await;
        assert_eq!(
            project_status,
            StatusCode::CREATED,
            "Failed to create test project: {project_body:?}"
        );
        let project_id = Uuid::parse_str(project_body["id"].as_str().unwrap()).unwrap();

        // Create a test location
        let location_data = json!({
            "name": format!("Test Location {}", test_suffix),
            "comment": "Test location for sample tests",
            "project_id": project_id
        });

        let location_response = app
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

        let (location_status, location_body) = extract_response_body(location_response).await;
        assert_eq!(
            location_status,
            StatusCode::CREATED,
            "Failed to create test location: {location_body:?}"
        );
        let location_id = Uuid::parse_str(location_body["id"].as_str().unwrap()).unwrap();

        (project_id, location_id)
    }

    #[tokio::test]
    async fn test_sample_crud_operations() {
        let app = setup_test_app().await;

        // Create dependencies
        let (_project_id, location_id) = create_test_project_and_location(&app, "CRUD").await;

        // Test creating a sample with valid enum values
        let sample_data = json!({
            "name": "Test Sample API",
            "type": "Bulk",
            "material_description": "Test material for API testing",
            "extraction_procedure": "Standard extraction via API",
            "filter_substrate": "Polycarbonate",
            "suspension_volume_litres": 0.050,
            "air_volume_litres": 100.0,
            "water_volume_litres": 0.200,
            "initial_concentration_gram_l": 0.001,
            "well_volume_litres": 0.0001,
            "remarks": "Created via API test suite",
            "longitude": -74.006000,
            "latitude": 40.712800,
            "location_id": location_id,
            "start_time": "2024-06-15T10:00:00Z",
            "stop_time": "2024-06-15T12:00:00Z",
            "flow_litres_per_minute": 2.0,
            "total_volume": 240.0,
            "treatments": [
                {
                    "id": "00000000-0000-0000-0000-000000000001",
                    "name": "heat",
                    "notes": "Heat treatment for API sample test",
                    "enzyme_volume_litres": 0.00005
                }
            ]
        });

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
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Failed to create sample: {body:?}"
        );

        // Validate response structure
        assert!(body["id"].is_string(), "Response should include ID");
        assert_eq!(body["name"], "Test Sample API");
        assert_eq!(body["type"], "Bulk");
        assert!(body["created_at"].is_string());
        assert!(body["treatments"].is_array());

        let sample_id = body["id"].as_str().unwrap();

        // Test getting the sample by ID
        let get_response = app
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

        let (get_status, get_body) = extract_response_body(get_response).await;
        assert_eq!(
            get_status,
            StatusCode::OK,
            "Failed to get sample: {get_body:?}"
        );
        assert_eq!(get_body["id"], sample_id);

        // Test getting all samples
        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/samples")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (list_status, list_body) = extract_response_body(list_response).await;
        assert_eq!(list_status, StatusCode::OK, "Failed to get samples");
        assert!(
            list_body.is_array(),
            "Samples list should be a direct array"
        );
    }

    #[tokio::test]
    async fn test_sample_type_validation() {
        let app = setup_test_app().await;

        // Create dependencies
        let (_project_id, location_id) =
            create_test_project_and_location(&app, "TYPE_VALIDATION").await;

        // Test valid sample types (using correct enum values)
        for (sample_type, expected_type) in [
            ("Bulk", "bulk"),
            ("Filter", "filter"),
            ("ProceduralBlank", "procedural_blank"),
            ("PureWater", "pure_water"),
        ] {
            let sample_data = json!({
                "name": format!("Test {} Sample", expected_type),
                "type": sample_type,
                "material_description": "Test material for validation",
                "location_id": location_id,
                "treatments": []
            });

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
            assert_eq!(
                status,
                StatusCode::CREATED,
                "Valid sample type {expected_type} should be accepted. Body: {body:?}"
            );
        }

        // Test invalid sample type
        let invalid_data = json!({
            "name": "Invalid Sample",
            "type": "invalid_type",
            "material_description": "Test material",
            "location_id": location_id,
            "treatments": []
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/samples")
                    .header("content-type", "application/json")
                    .body(Body::from(invalid_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, _body) = extract_response_body(response).await;
        assert!(
            status.is_client_error(),
            "Invalid sample type should be rejected"
        );
    }

    #[tokio::test]
    async fn test_sample_filtering() {
        let app = setup_test_app().await;

        // Create dependencies
        let (_project_id, location_id) = create_test_project_and_location(&app, "FILTERING").await;

        // Create test samples for filtering
        let sample_types = [("Bulk", "bulk"), ("Filter", "filter")];
        for (input_type, display_type) in sample_types {
            let sample_data = json!({
                "name": format!("Filter Test {} Sample", display_type),
                "type": input_type,
                "material_description": "Test material for filtering",
                "location_id": location_id,
                "treatments": []
            });

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

            let (status, _) = extract_response_body(response).await;
            assert_eq!(status, StatusCode::CREATED);
        }

        // Test filtering by type
        let filter_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/samples?filter[type]=bulk")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (filter_status, filter_body) = extract_response_body(filter_response).await;
        assert_eq!(
            filter_status,
            StatusCode::OK,
            "Type filtering should work: {filter_body:?}"
        );

        // Test sorting by created_at
        let sort_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/samples?sort[created_at]=desc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (sort_status, _) = extract_response_body(sort_response).await;
        assert_eq!(sort_status, StatusCode::OK, "Sorting should work");
    }

    #[tokio::test]
    async fn test_treatment_enum_validation() {
        let app = setup_test_app().await;

        // Create dependencies
        let (_project_id, location_id) =
            create_test_project_and_location(&app, "TREATMENT_VALIDATION").await;

        // Test valid treatment enum values
        for treatment_name in ["none", "heat", "h2o2"] {
            let enzyme_volume = if treatment_name == "h2o2" {
                serde_json::Value::String("0.00005".to_string())
            } else {
                serde_json::Value::Null
            };

            let sample_data = json!({
                "name": format!("Treatment Test {} Sample", treatment_name),
                "type": "Bulk",
                "material_description": "Test material for treatment validation",
                "location_id": location_id,
                "treatments": [
                    {
                        "id": "00000000-0000-0000-0000-000000000001",
                        "name": treatment_name,
                        "notes": format!("Test {} treatment", treatment_name),
                        "enzyme_volume_litres": enzyme_volume
                    }
                ]
            });

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
            assert_eq!(
                status,
                StatusCode::CREATED,
                "Valid treatment {treatment_name} should be accepted. Body: {body:?}"
            );
        }
    }
}
