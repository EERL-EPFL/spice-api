use super::models::{Campaign, CampaignCreate, CampaignUpdate};
use crate::common::auth::Role;
use crate::common::state::AppState;
use axum::{extract::Extension, response::IntoResponse};
use axum_keycloak_auth::{PassthroughMode, decode::KeycloakToken, layer::KeycloakAuthLayer};
use crudcrate::{CRUDResource, crud_handlers};
use utoipa_axum::{router::OpenApiRouter, routes};

crud_handlers!(Campaign, CampaignUpdate, CampaignCreate);

pub fn router(state: &AppState) -> OpenApiRouter
where
    Campaign: CRUDResource,
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
    } else {
        println!(
            "Warning: Mutating routes of {} router are not protected",
            Campaign::RESOURCE_NAME_PLURAL
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
    (StatusCode::OK, "Token debug information printed to console")
}

#[cfg(test)]
mod tests {
    use crate::config::test_helpers::setup_test_app;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_campaign_crud_operations() {
        let app = setup_test_app().await;

        // Test creating a campaign
        let campaign_data = json!({
            "name": "Test Campaign API",
            "comment": "Campaign created via API test",
            "start_date": "2024-06-01T08:00:00Z",
            "end_date": "2024-12-31T18:00:00Z"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/campaigns")
                    .header("content-type", "application/json")
                    .body(Body::from(campaign_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(response.status().is_success(), "Failed to create campaign");

        // Test getting all campaigns
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/campaigns")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK, "Failed to get campaigns");
    }

    #[tokio::test]
    async fn test_campaign_validation() {
        let app = setup_test_app().await;

        // Test creating campaign with invalid data
        let invalid_data = json!({
            "name": "", // Empty name should be invalid
            "comment": "Invalid campaign"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/campaigns")
                    .header("content-type", "application/json")
                    .body(Body::from(invalid_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response.status().is_client_error(),
            "Should reject invalid data"
        );
    }

    #[tokio::test]
    async fn test_campaign_filtering_and_pagination() {
        let app = setup_test_app().await;

        // Test pagination
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/campaigns?limit=5&offset=0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK, "Pagination should work");

        // Test sorting
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/campaigns?sort[name]=asc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK, "Sorting should work");
    }
}
