use crate::config::Config;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Processing status for async operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(ToSchema, Deserialize, Serialize, Default)]
pub struct UIConfiguration {
    // pub keycloak: Keycloak, // DIsable for now (this is the structure of the BFF)
    #[serde(rename = "clientId")]
    pub client_id: String,
    pub realm: String,
    pub url: String,
    pub deployment: String,
}

impl UIConfiguration {
    pub fn new() -> Self {
        let config: Config = Config::from_env();
        Self {
            client_id: config.keycloak_ui_id,
            realm: config.keycloak_realm,
            url: config.keycloak_url,
            deployment: config.deployment,
        }
    }
}

#[derive(ToSchema, Deserialize, Serialize)]
pub struct HealthCheck {
    pub status: String,
}
