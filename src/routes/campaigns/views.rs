use super::models::{Location, LocationCreate, LocationUpdate};
use crate::common::auth::Role;
use crate::common::state::AppState;
use axum::{extract::Extension, response::IntoResponse};
use axum_keycloak_auth::{PassthroughMode, decode::KeycloakToken, layer::KeycloakAuthLayer};
use crudcrate::{CRUDResource, crud_handlers};
use sea_orm::ConnectionTrait;
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
