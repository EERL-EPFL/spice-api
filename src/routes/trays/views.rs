use super::models::{TrayConfiguration, TrayConfigurationCreate, TrayConfigurationUpdate};
use crate::common::auth::Role;
use crate::common::state::AppState;
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::{CRUDResource, crud_handlers};
use utoipa_axum::{router::OpenApiRouter, routes};

crud_handlers!(
    TrayConfiguration,
    TrayConfigurationUpdate,
    TrayConfigurationCreate
);

pub fn router(state: &AppState) -> OpenApiRouter
where
    TrayConfiguration: CRUDResource,
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
    } else {
        println!(
            "Warning: Mutating routes of {} router are not protected",
            TrayConfiguration::RESOURCE_NAME_PLURAL
        );
    }

    mutating_router
}

#[cfg(test)]
mod tests {
    use crate::config::test_helpers::{cleanup_test_data, setup_test_app, setup_test_db};
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_tray_configuration_validation() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;

        // Clean up any existing test data
        cleanup_test_data(&db).await;

        // Test creating tray config with empty name
        let invalid_data = json!({
            "name": null,
            "experiment_default": false
        });

        let response = app
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
        // convert to &str for printing
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body aggregation failed");

        // convert to &str for printing
        let body_str: &str = std::str::from_utf8(&bytes).expect("body was not valid UTF-8");
        assert!(
            status.is_client_error(),
            "Should reject tray configuration with empty name. Status: {status:?} body: {body_str:?}"
        );

        // Clean up after test
        cleanup_test_data(&db).await;
    }
}
