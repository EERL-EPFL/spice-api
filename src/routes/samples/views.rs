use super::models::{Sample, SampleCreate, SampleUpdate};
use crate::common::auth::Role;
use axum_keycloak_auth::{
    PassthroughMode, instance::KeycloakAuthInstance, layer::KeycloakAuthLayer,
};
use crudcrate::{CRUDResource, crud_handlers};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};

crud_handlers!(Sample, SampleUpdate, SampleCreate);

pub fn router(
    db: &DatabaseConnection,
    keycloak_auth_instance: Option<Arc<KeycloakAuthInstance>>,
) -> OpenApiRouter
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
        .with_state(db.clone());

    if let Some(instance) = keycloak_auth_instance {
        mutating_router = mutating_router.layer(
            KeycloakAuthLayer::<Role>::builder()
                .instance(instance)
                .passthrough_mode(PassthroughMode::Block)
                .persist_raw_claims(false)
                .expected_audiences(vec![String::from("account")])
                .required_roles(vec![Role::Administrator])
                .build(),
        );
    } else {
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
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_sample_crud_operations() {
        let app = setup_test_app().await;

        // Test creating a sample
        let sample_data = json!({
            "name": "Test Sample API",
            "type": "bulk",
            "material_description": "Test material for API testing",
            "extraction_procedure": "Standard extraction via API",
            "filter_substrate": "Polycarbonate",
            "suspension_volume_liters": 0.050,
            "air_volume_liters": 100.0,
            "water_volume_liters": 0.200,
            "initial_concentration_gram_l": 0.001,
            "well_volume_liters": 0.0001,
            "background_region_key": "BG_API_TEST",
            "remarks": "Created via API test suite",
            "longitude": -74.006_000,
            "latitude": 40.712_800,
            "start_time": "2024-06-15T10:00:00Z",
            "stop_time": "2024-06-15T12:00:00Z",
            "flow_litres_per_minute": 2.0,
            "total_volume": 240.0
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

        assert!(response.status().is_success(), "Failed to create sample");

        // Test getting all samples
        let response = app
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

        assert_eq!(response.status(), StatusCode::OK, "Failed to get samples");
    }

    #[tokio::test]
    async fn test_sample_type_validation() {
        let app = setup_test_app().await;

        // Test valid sample types
        for sample_type in ["bulk", "filter", "procedural_blank", "pure_water"] {
            let sample_data = json!({
                "name": format!("Test {} Sample", sample_type),
                "type": sample_type,
                "material_description": "Test material"
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

            assert!(
                response.status().is_success(),
                "Valid sample type {sample_type} should be accepted",
            );
        }

        // Test invalid sample type
        let invalid_data = json!({
            "name": "Invalid Sample",
            "type": "invalid_type",
            "material_description": "Test material"
        });

        let response = app
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

        assert!(
            response.status().is_client_error(),
            "Invalid sample type should be rejected"
        );
    }

    #[tokio::test]
    async fn test_sample_filtering() {
        let app = setup_test_app().await;

        // Test filtering by type
        let response = app
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

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Type filtering should work"
        );

        // Test sorting by created_at
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/samples?sort[created_at]=desc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK, "Sorting should work");
    }
}
