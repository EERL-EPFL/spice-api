use super::models::HealthCheck;
use super::models::UIConfiguration;
use crate::common::state::AppState;
use axum::{Json, extract::State, http::StatusCode};
use sea_orm::DatabaseConnection;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router(state: &AppState) -> OpenApiRouter {
    // let db = app_state.db.clone();
    OpenApiRouter::new()
        .routes(routes!(healthz))
        .routes(routes!(get_ui_config))
        .with_state(state.db.clone())
}

#[utoipa::path(
    get,
    path = "/healthz",
    responses(
        (
            status = OK,
            description = "Kubernetes health check",
            body = str,
            content_type = "text/plain"
        )
    )
)]
pub async fn healthz(State(db): State<DatabaseConnection>) -> (StatusCode, Json<HealthCheck>) {
    if db.ping().await.is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(HealthCheck {
                status: "error".to_string(),
            }),
        );
    }

    (
        StatusCode::OK,
        Json(HealthCheck {
            status: "ok".to_string(),
        }),
    )
}

#[utoipa::path(
    get,
    path = "/api/config",
    responses(
        (
            status = OK,
            description = "Web UI configuration",
            body = str,
            content_type = "text/plain"
        )
    )
)]
pub async fn get_ui_config() -> Json<UIConfiguration> {
    Json(UIConfiguration::new())
}
