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
    pub tests_running: bool, // Flag to indicate if tests are running
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
            tests_running: false, // Always false if using Config from_env
            db_url,
        }
    }

    #[cfg(test)]
    pub fn for_tests() -> Self {
        // Set default test environment variables if not already set
        let db_url = Some(format!(
            "{}://{}:{}@{}:{}/{}",
            env::var("DB_PREFIX").unwrap_or_else(|_| "postgresql".to_string()),
            env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
            env::var("DB_PASSWORD").unwrap_or_else(|_| "psql".to_string()),
            env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string()),
            env::var("DB_NAME").unwrap_or_else(|_| "spice_test".to_string())
        ));

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
            tests_running: true, // Set to true for test configurations
            db_url,
        }
    }
}

#[cfg(test)]
pub mod test_helpers {
    use super::*;
    use crate::routes::build_router;
    use axum::Router;
    use migration::{Migrator, MigratorTrait};
    use sea_orm::{Database, DatabaseConnection};

    pub fn init_test_env() {
        // No need for Once since each test gets its own database
        Config::for_tests();
    }

    pub async fn setup_test_db() -> DatabaseConnection {
        init_test_env();

        // Use proper SQLite in-memory database connection string
        // Each connection to :memory: creates a separate database instance
        let database_url = "sqlite::memory:";

        println!("Creating new in-memory SQLite database: {database_url}");

        let db = Database::connect(database_url)
            .await
            .expect("Failed to connect to SQLite test database");

        // Test the connection
        if let Err(e) = db.ping().await {
            panic!("SQLite database connection failed: {e:?}");
        }

        // Run migrations to create all tables
        Migrator::up(&db, None)
            .await
            .expect("Failed to run database migrations");

        println!("SQLite test database ready with all tables created");
        db
    }

    pub async fn setup_test_app() -> Router {
        let db = setup_test_db().await;
        let mut config = Config::for_tests();
        // Disable Keycloak for tests by setting the URL to empty
        config.keycloak_url = String::new();
        build_router(&db, &config)
    }

    // No cleanup needed for in-memory SQLite - it's automatically destroyed
    pub async fn cleanup_test_data(_db: &DatabaseConnection) {
        // In-memory SQLite databases are automatically cleaned up when the connection is dropped
        // This function is kept for API compatibility but does nothing
    }
}
