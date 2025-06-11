use super::models::{Asset, AssetCreate, AssetUpdate};
use crate::common::auth::Role;
use crate::common::state::AppState;
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::{CRUDResource, crud_handlers};
use utoipa_axum::{router::OpenApiRouter, routes};

crud_handlers!(Asset, AssetUpdate, AssetCreate);

pub fn router(state: &AppState) -> OpenApiRouter
where
    Asset: CRUDResource,
{
    let mut mutating_router = OpenApiRouter::new()
        .routes(routes!(get_one_handler))
        .routes(routes!(get_all_handler))
        .routes(routes!(create_one_handler))
        .routes(routes!(update_one_handler))
        .routes(routes!(delete_one_handler))
        .routes(routes!(delete_many_handler))
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
    } else {
        println!(
            "Warning: Mutating routes of {} router are not protected",
            Asset::RESOURCE_NAME_PLURAL
        );
    }

    mutating_router
}

#[cfg(test)]
mod tests {
    use crate::config::test_helpers::{cleanup_test_data, setup_test_app, setup_test_db};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_asset_crud_operations() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;

        // Clean up any existing test data
        cleanup_test_data(&db).await;

        // Test creating an asset
        let asset_data = json!({
            "original_filename": "test_data.csv",
            "s3_key": "test/data/test_data.csv",
            "size_bytes": 1024,
            "uploaded_by": "test@example.com",
            "uploaded_at": "2024-06-20T15:00:00Z",
            "is_deleted": false,
            "type": "data",
            "role": "results"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/assets")
                    .header("content-type", "application/json")
                    .body(Body::from(asset_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(response.status().is_success(), "Failed to create asset");

        // Test getting all assets
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/assets")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK, "Failed to get assets");

        // Clean up after test
        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_asset_filtering() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;

        // Clean up any existing test data
        cleanup_test_data(&db).await;

        // Test filtering by type
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/assets?filter[type]=data")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Type filtering should work"
        );

        // Test filtering by role
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/assets?filter[role]=results")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Role filtering should work"
        );

        // Test filtering by deleted status
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/assets?filter[is_deleted]=false")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Deleted status filtering should work"
        );

        // Clean up after test
        cleanup_test_data(&db).await;
    }

    #[tokio::test]
    async fn test_asset_type_validation() {
        let db = setup_test_db().await;
        let app = setup_test_app().await;

        // Clean up any existing test data
        cleanup_test_data(&db).await;

        // Test valid asset types
        for asset_type in ["data", "image", "log", "report", "protocol"] {
            let asset_data = json!({
                "original_filename": format!("test_{}.file", asset_type),
                "s3_key": format!("test/{}/file", asset_type),
                "type": asset_type,
                "size_bytes": 100,
                "uploaded_at": "2024-06-20T15:00:00Z",
                "is_deleted": false
            });

            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/assets")
                        .header("content-type", "application/json")
                        .body(Body::from(asset_data.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert!(
                response.status().is_success(),
                "Valid asset type {asset_type} should be accepted"
            );
        }

        // Clean up after test
        cleanup_test_data(&db).await;
    }
}
