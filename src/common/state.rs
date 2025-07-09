use crate::config::Config;
use crate::services::data_processing_service::DataProcessingService;
use axum_keycloak_auth::instance::KeycloakAuthInstance;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Config,
    pub keycloak_auth_instance: Option<Arc<KeycloakAuthInstance>>,
    pub data_processing_service: DataProcessingService,
}

impl AppState {
    pub fn new(
        db: DatabaseConnection,
        config: Config,
        keycloak_auth_instance: Option<Arc<KeycloakAuthInstance>>,
    ) -> Self {
        let data_processing_service = DataProcessingService::new(db.clone());

        Self {
            db,
            config,
            keycloak_auth_instance,
            data_processing_service,
        }
    }
}
