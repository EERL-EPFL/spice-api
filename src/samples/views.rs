pub use super::models::{Sample, router as crudrouter};
use crate::common::auth::Role;
use crate::common::state::AppState;
use axum::extract::{Path, State};
use axum::response::Json;
use axum::routing::get;
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::CRUDResource;
use sea_orm::{ColumnTrait, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect, RelationTrait};
use serde_json::Value;
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

pub fn router(state: &AppState) -> OpenApiRouter
where
    Sample: CRUDResource,
{
    let mut mutating_router = crudrouter(&state.db.clone());

    mutating_router = mutating_router.route(
        "/{id}/experiments",
        get(get_sample_experiments).with_state(state.clone()),
    );

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
            Sample::RESOURCE_NAME_PLURAL
        );
    }

    mutating_router
}

/// Get all experiments for a specific sample
/// Returns experiments related to this sample via treatments -> regions -> experiments
#[utoipa::path(
    get,
    path = "/samples/{id}/experiments",
    params(
        ("id" = Uuid, Path, description = "Sample ID to fetch experiments for")
    ),
    responses(
        (status = 200, description = "List of experiments for this sample", body = Vec<crate::experiments::models::Experiment>),
        (status = 404, description = "Sample not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "samples",
    summary = "Get sample experiments",
    description = "Retrieve all experiments related to a specific sample via the relationship chain: sample -> treatments -> regions -> experiments"
)]
pub async fn get_sample_experiments(
    Path(sample_id): Path<Uuid>,
    State(app_state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let db = &app_state.db;

    let experiments = crate::experiments::models::Entity::find()
        .join(
            JoinType::InnerJoin,
            crate::experiments::models::Relation::Regions.def(),
        )
        .join(
            JoinType::InnerJoin,
            crate::tray_configurations::regions::models::Relation::Treatments.def(),
        )
        .filter(crate::treatments::models::Column::SampleId.eq(sample_id))
        .distinct()
        .order_by_desc(crate::experiments::models::Column::PerformedAt)
        .all(db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            )
        })?;

    let experiment_data: Vec<crate::experiments::models::Experiment> =
        experiments.into_iter().map(Into::into).collect();

    Ok(Json(serde_json::to_value(experiment_data).map_err(
        |e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Serialization error: {e}"),
            )
        },
    )?))
}
