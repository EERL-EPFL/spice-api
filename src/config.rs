use dotenvy::dotenv;
use serde::Deserialize;
use std::env;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub db_url: Option<String>,
    pub app_name: String,
    pub keycloak_ui_id: String,
    pub keycloak_url: String,
    pub keycloak_realm: String,
    pub deployment: String,
    pub admin_role: String,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok(); // Load from .env file if available
        let db_url = env::var("DB_URL").ok().or_else(|| {
            Some(format!(
                "{}://{}:{}@{}:{}/{}",
                env::var("DB_PREFIX").unwrap_or_else(|_| "postgresql".to_string()),
                env::var("DB_USER").expect("DB_USER must be set"),
                env::var("DB_PASSWORD").expect("DB_PASSWORD must be set"),
                env::var("DB_HOST").expect("DB_HOST must be set"),
                env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string()),
                env::var("DB_NAME").expect("DB_NAME must be set"),
            ))
        });

        Config {
            app_name: env::var("APP_NAME").expect("APP_NAME must be set"),
            keycloak_ui_id: env::var("KEYCLOAK_UI_ID").expect("KEYCLOAK_UI_ID must be set"),
            keycloak_url: env::var("KEYCLOAK_URL").expect("KEYCLOAK_URL must be set"),
            keycloak_realm: env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
            deployment: env::var("DEPLOYMENT")
                .expect("DEPLOYMENT must be set, this can be local, dev, stage, or prod"),
            admin_role: "spice-admin".to_string(), // Admin role name in Keycloak
            s3_access_key: env::var("S3_ACCESS_KEY").ok(),
            s3_secret_key: env::var("S3_SECRET_KEY").ok(),
            s3_bucket: env::var("S3_BUCKET").ok(),
            s3_url: env::var("S3_URL").ok(),
            db_url,
        }
    }
}
