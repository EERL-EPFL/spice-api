mod common;
mod config;
mod external;
mod routes;
mod services;

mod assets;
mod experiments;
mod locations;
mod nucleation_events;
mod projects;
mod samples;
mod tray_configurations;
mod treatments;

use crate::config::Config;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};

#[tokio::main]
async fn main() {
    // Set up tracing/logging
    tracing_subscriber::fmt::init();
    println!("Starting server...");

    // Load configuration and environment variables to pass to the application
    let config: Config = Config::from_env();

    let db: DatabaseConnection = Database::connect(config.db_url.as_ref().unwrap())
        .await
        .unwrap();

    if db.ping().await.is_ok() {
        println!("Connected to the database");
    } else {
        println!("Could not connect to the database");
    }

    // Run migrations
    Migrator::up(&db, None)
        .await
        .expect("Failed to run migrations");

    // Left commented here in case of need to downgrade
    // Migrator::down(&db, Some(1)).await.expect("Failed to run downgrade migration");

    println!("DB migrations complete");

    println!(
        "Starting server {} ({} deployment) ...",
        config.app_name,
        config.deployment.to_uppercase()
    );

    // crudcrate::analyse_all_registered_models(&db, true).await.unwrap();

    let addr: std::net::SocketAddr = "0.0.0.0:3000".parse().unwrap();
    println!("Listening on {addr}");

    let router = routes::build_router(&db, &config);

    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        router.into_make_service(),
    )
    .await
    .unwrap();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_socket_address_parsing() {
        // Test that the socket address used in main can be parsed
        let addr: Result<std::net::SocketAddr, _> = "0.0.0.0:3000".parse();
        assert!(addr.is_ok());

        let addr = addr.unwrap();
        assert_eq!(addr.port(), 3000);
        assert!(addr.ip().is_unspecified()); // 0.0.0.0 is unspecified
    }

    #[test]
    fn test_alternative_socket_addresses() {
        // Test other valid socket addresses that could be used
        let test_addresses = vec!["127.0.0.1:3000", "0.0.0.0:8080", "localhost:3000"];

        for addr_str in test_addresses {
            let addr: Result<std::net::SocketAddr, _> = addr_str.parse();
            if addr_str != "localhost:3000" {
                // localhost requires name resolution which might fail in test env
                assert!(addr.is_ok(), "Failed to parse address: {addr_str}");
            }
        }
    }

    #[test]
    fn test_invalid_socket_addresses() {
        // Test that invalid socket addresses fail to parse
        let invalid_addresses = vec![
            "invalid",
            "256.256.256.256:3000", // Invalid IP
            "0.0.0.0:99999",        // Invalid port (too high)
            "0.0.0.0:-1",           // Invalid port (negative)
            "",                     // Empty string
        ];

        for addr_str in invalid_addresses {
            let addr: Result<std::net::SocketAddr, _> = addr_str.parse();
            assert!(addr.is_err(), "Expected parsing to fail for: {addr_str}");
        }
    }

    #[tokio::test]
    async fn test_database_connection_concept() {
        // Test database connection concepts without requiring actual DB
        use crate::config::Config;

        let config = Config::for_tests();

        // Test that we can create a database URL (even if it might not be valid)
        if let Some(db_url) = config.db_url.as_ref() {
            assert!(!db_url.is_empty());
            // Basic URL structure validation
            assert!(db_url.contains("://"));
        }
    }

    #[test]
    fn test_app_configuration_concepts() {
        // Test application configuration concepts
        use crate::config::Config;

        let config = Config::for_tests();

        // Test that config has required fields
        assert!(!config.app_name.is_empty());

        // Test that deployment can be formatted to uppercase
        let deployment_upper = config.deployment.to_uppercase();
        assert!(!deployment_upper.is_empty());
    }
}
