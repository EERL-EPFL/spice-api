pub mod assets;
pub mod experiments;
pub mod locations;
pub mod projects;
pub mod samples;
pub mod tray_configurations;
pub mod treatments;

use crate::common::state::AppState;
use crate::config::Config;
use axum::{Router, extract::DefaultBodyLimit};
use axum_keycloak_auth::{Url, instance::KeycloakAuthInstance, instance::KeycloakConfig};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

pub fn build_router(db: &DatabaseConnection, config: &Config) -> Router {
    #[derive(OpenApi)]
    #[openapi(
        modifiers(&SecurityAddon),
        security(
            ("bearerAuth" = [])
        )
    )]
    struct ApiDoc;

    struct SecurityAddon;

    impl utoipa::Modify for SecurityAddon {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            if let Some(components) = openapi.components.as_mut() {
                components.add_security_scheme(
                    "bearerAuth",
                    utoipa::openapi::security::SecurityScheme::Http(
                        utoipa::openapi::security::HttpBuilder::new()
                            .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                            .bearer_format("JWT")
                            .build(),
                    ),
                );
            }
        }
    }

    let keycloak_instance: Option<Arc<KeycloakAuthInstance>> = if config.keycloak_url.is_empty() {
        // Skip Keycloak initialization for tests
        None
    } else {
        Some(Arc::new(KeycloakAuthInstance::new(
            KeycloakConfig::builder()
                .server(Url::parse(&config.keycloak_url).unwrap())
                .realm(String::from(&config.keycloak_realm))
                .build(),
        )))
    };

    let app_state: AppState = AppState::new(db.clone(), config.clone(), keycloak_instance);

    // Build the router with routes from the plots module
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(crate::common::views::router(&app_state)) // Root routes
        .nest("/api/locations", locations::views::router(&app_state))
        .nest("/api/projects", projects::views::router(&app_state))
        .nest("/api/experiments", experiments::views::router(&app_state))
        // .nest("/api", phase_changes::router(&app_state))
        .nest("/api/samples", samples::views::router(&app_state))
        .nest("/api/assets", assets::views::router(&app_state))
        .nest("/api/trays", tray_configurations::views::router(&app_state))
        .nest("/api/treatments", treatments::views::router(&app_state))
        // .nest(
        //     "/api",
        //     freezing_results::views::freezing_results_routes().with_state(app_state.clone()),
        // )
        .layer(DefaultBodyLimit::max(30 * 1024 * 1024))
        .split_for_parts();

    // Merge the Excel upload routes separately since they're not OpenApiRouter compatible
    router
        // .nest(
        // "/api/experiments",
        // experiments::views::excel_upload_router().with_state(app_state),
        // )
        .merge(Scalar::with_url("/api/docs", api))
}
