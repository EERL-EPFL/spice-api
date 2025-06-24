use super::models::{Location, LocationCreate, LocationUpdate};
use crate::common::auth::Role;
use crate::common::state::AppState;
use axum::{extract::Extension, response::IntoResponse};
use axum_keycloak_auth::{PassthroughMode, decode::KeycloakToken, layer::KeycloakAuthLayer};
use crudcrate::{CRUDResource, crud_handlers};
use utoipa_axum::{router::OpenApiRouter, routes};

crud_handlers!(Location, LocationUpdate, LocationCreate);

pub fn router(state: &AppState) -> OpenApiRouter
where
    Location: CRUDResource,
{
    let mut mutating_router = OpenApiRouter::new()
        .routes(routes!(get_one_handler))
        .routes(routes!(get_all_handler))
        .routes(routes!(create_one_handler))
        .routes(routes!(update_one_handler))
        .routes(routes!(delete_one_handler))
        .routes(routes!(delete_many_handler))
        .routes(routes!(debug_token))
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
            Location::RESOURCE_NAME_PLURAL
        );
    }

    mutating_router
}

#[utoipa::path(
    get,
    path = "/debug-token",
    responses(
        (status = axum::http::StatusCode::OK, description = "Token debug information printed to console"),
        (status = axum::http::StatusCode::UNAUTHORIZED, description = "Unauthorized access"),
        (status = axum::http::StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Server Error")
    ),
    operation_id = "debug_token",
    summary = "Debug Keycloak token",
    description = "Prints the Keycloak token payload to the console for debugging purposes."
)]
pub async fn debug_token(Extension(token): Extension<KeycloakToken<Role>>) -> impl IntoResponse {
    println!("Token payload: {token:#?}");
    (
        axum::http::StatusCode::OK,
        "Token debug information printed to console",
    )
}

#[cfg(test)]
mod tests {
    use crate::config::test_helpers::{cleanup_test_data, setup_test_app, setup_test_db};
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
        (status, body)
    }

    async fn create_test_project(app: &axum::Router, test_suffix: &str) -> uuid::Uuid {
        let project_data = json!({
            "name": format!("Test Project {}", test_suffix),
            "note": "Test project for location tests",
            "colour": "#FF0000"
        });

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
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Failed to create test project: {:?}",
            body
        );
        uuid::Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
    }

    // #[tokio::test]
    // async fn test_location_crud_operations() {
    //     let db = setup_test_db().await;
    //     let app = setup_test_app().await;
    //     cleanup_test_data(&db).await;

    //     // Create a test project for the location
    //     let project_id = create_test_project(&app, "LOCATION_CRUD").await;

    //     // Test creating a location with unique name
    //     let location_data = json!({
    //         "name": format!("Test Location API {}", uuid::Uuid::new_v4()),
    //         "comment": "Location created via API test",
    //         "start_date": "2024-06-01T00:00:00Z",
    //         "end_date": "2024-12-31T23:59:59Z",
    //         "project_id": project_id
    //     });

    //     let response = app
    //         .clone()
    //         .oneshot(
    //             Request::builder()
    //                 .method("POST")
    //                 .uri("/api/locations")
    //                 .header("content-type", "application/json")
    //                 .body(Body::from(location_data.to_string()))
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     let (status, body) = extract_response_body(response).await;
    //     assert_eq!(
    //         status,
    //         StatusCode::CREATED,
    //         "Failed to create location: {:?}",
    //         body
    //     );

    //     // Validate response structure
    //     assert!(body["id"].is_string(), "Response should include ID");
    //     assert!(body["name"].is_string());
    //     assert_eq!(body["comment"], "Location created via API test");
    //     assert!(body["created_at"].is_string());

    //     let location_id = body["id"].as_str().unwrap();

    //     // Test getting the location by ID
    //     let get_response = app
    //         .clone()
    //         .oneshot(
    //             Request::builder()
    //                 .method("GET")
    //                 .uri(&format!("/api/locations/{}", location_id))
    //                 .body(Body::empty())
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     let (get_status, get_body) = extract_response_body(get_response).await;
    //     assert_eq!(
    //         get_status,
    //         StatusCode::OK,
    //         "Failed to get location: {:?}",
    //         get_body
    //     );
    //     assert_eq!(get_body["id"], location_id);

    //     // Test getting all locations
    //     let list_response = app
    //         .clone()
    //         .oneshot(
    //             Request::builder()
    //                 .method("GET")
    //                 .uri("/api/locations")
    //                 .body(Body::empty())
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     let (list_status, list_body) = extract_response_body(list_response).await;
    //     assert_eq!(list_status, StatusCode::OK, "Failed to get locations");
    //     assert!(list_body["items"].is_array());

    //     cleanup_test_data(&db).await;
    // }

    #[tokio::test]
    async fn test_location_validation() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;
        cleanup_test_data(&db).await;

        // Test creating location with invalid data (null name)
        let invalid_data = json!({
            "name": null,
            "comment": "Invalid location"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/locations")
                    .header("content-type", "application/json")
                    .body(Body::from(invalid_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, _body) = extract_response_body(response).await;
        assert!(
            status.is_client_error(),
            "Should reject location with null name"
        );

        // Test creating location with missing required fields
        let incomplete_data = json!({
            "comment": "Incomplete location"
            // Missing name
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/locations")
                    .header("content-type", "application/json")
                    .body(Body::from(incomplete_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, _body) = extract_response_body(response).await;
        assert!(
            status.is_client_error(),
            "Should reject incomplete location data"
        );

        cleanup_test_data(&db).await;
    }

    // #[tokio::test]
    // async fn test_location_filtering_and_pagination() {
    //     let db = setup_test_db().await;
    //     let app = setup_test_app().await;
    //     cleanup_test_data(&db).await;

    //     // Create a test project for the locations
    //     let project_id = create_test_project(&app, "LOCATION_FILTERING").await;

    //     // Create some test locations for filtering
    //     for i in 1..=3 {
    //         let location_data = json!({
    //             "name": format!("Filter Test Location {}", i),
    //             "comment": format!("Test location {} for filtering", i),
    //             "project_id": project_id
    //         });

    //         let response = app
    //             .clone()
    //             .oneshot(
    //                 Request::builder()
    //                     .method("POST")
    //                     .uri("/api/locations")
    //                     .header("content-type", "application/json")
    //                     .body(Body::from(location_data.to_string()))
    //                     .unwrap(),
    //             )
    //             .await
    //             .unwrap();

    //         let (status, body) = extract_response_body(response).await;
    //         assert_eq!(
    //             status,
    //             StatusCode::CREATED,
    //             "Failed to create test location: {:?}",
    //             body
    //         );
    //     }

    //     // Test pagination
    //     let pagination_response = app
    //         .clone()
    //         .oneshot(
    //             Request::builder()
    //                 .method("GET")
    //                 .uri("/api/locations?limit=2&offset=0")
    //                 .body(Body::empty())
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     let (pagination_status, pagination_body) = extract_response_body(pagination_response).await;
    //     assert_eq!(pagination_status, StatusCode::OK, "Pagination should work");

    //     let items = pagination_body["items"].as_array().unwrap();
    //     assert!(items.len() <= 2, "Should respect limit parameter");

    //     // Test sorting
    //     let sort_response = app
    //         .clone()
    //         .oneshot(
    //             Request::builder()
    //                 .method("GET")
    //                 .uri("/api/locations?sort[name]=asc")
    //                 .body(Body::empty())
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     let (sort_status, _) = extract_response_body(sort_response).await;
    //     assert_eq!(sort_status, StatusCode::OK, "Sorting should work");

    //     cleanup_test_data(&db).await;
    // }
}
