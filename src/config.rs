use dotenvy::dotenv;
use serde::Deserialize;
use std::env;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub db_url: Option<String>,
    pub app_name: String,
    pub keycloak_ui_id: String,
    pub keycloak_url: String,
    pub keycloak_realm: String,
    pub deployment: String,
    pub admin_role: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_bucket_id: String,
    pub s3_url: String,
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
            s3_access_key: env::var("S3_ACCESS_KEY").expect("S3_ACCESS_KEY must be set"),
            s3_secret_key: env::var("S3_SECRET_KEY").expect("S3_SECRET_KEY must be set"),
            s3_bucket_id: env::var("S3_BUCKET_ID").expect("S3_BUCKET must be set"),
            s3_url: env::var("S3_URL").expect("S3_URL must be set"),
            db_url,
        }
    }

    #[cfg(test)]
    pub fn for_tests() -> Self {
        // Set default test environment variables if not already set
        Config {
            app_name: "spice-api-test".to_string(),
            keycloak_ui_id: "test-ui".to_string(),
            keycloak_url: "http://localhost:8080".to_string(),
            keycloak_realm: "test-realm".to_string(),
            deployment: "test".to_string(),
            admin_role: "spice-admin".to_string(),
            s3_access_key: "test-access-key".to_string(),
            s3_secret_key: "test-secret-key".to_string(),
            s3_bucket_id: "test-bucket".to_string(),
            s3_url: "http://localhost:9000".to_string(),
            db_url: None,
        }
    }
}

#[cfg(test)]
pub mod test_helpers {
    use super::*;
    use crate::routes::build_router;
    use axum::Router;
    use sea_orm::{Database, DatabaseConnection};
    use std::sync::Once;

    static INIT: Once = Once::new();

    pub fn init_test_env() {
        INIT.call_once(|| {
            // Initialize test configuration
            Config::for_tests();
        });
    }

    pub async fn setup_test_db() -> DatabaseConnection {
        init_test_env();

        let database_url = format!(
            "{}://{}:{}@{}:{}/{}",
            env::var("DB_PREFIX").unwrap_or_else(|_| "postgresql".to_string()),
            env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
            env::var("DB_PASSWORD").unwrap_or_else(|_| "psql".to_string()),
            env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string()),
            env::var("DB_NAME").unwrap_or_else(|_| "spice_test".to_string())
        );

        Database::connect(database_url)
            .await
            .expect("Failed to connect to test database")
    }

    pub async fn setup_test_app() -> Router {
        let db = setup_test_db().await;
        let config = Config::for_tests();
        build_router(&db, &config)
    }
}
