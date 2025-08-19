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
    // Migrator::down(&db, Some(1))  // Downgrade one migration step
    //     .await
    //     .expect("Failed to run downgrade migration");

    println!("DB migrations complete");

    println!(
        "Starting server {} ({} deployment) ...",
        config.app_name,
        config.deployment.to_uppercase()
    );

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
