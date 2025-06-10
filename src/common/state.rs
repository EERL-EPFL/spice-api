use crate::config::Config;
use axum_keycloak_auth::instance::KeycloakAuthInstance;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Config,
    pub keycloak_auth_instance: Option<Arc<KeycloakAuthInstance>>,
}
