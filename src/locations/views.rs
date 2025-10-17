use super::models::{Location, router as crudrouter};
use crate::common::auth::Role;
use crate::common::state::AppState;
use axum::extract::{Path, State};
use axum::response::Json;
use axum::routing::get;
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};
use crudcrate::CRUDResource;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Statement};
use serde_json::{Value, json};
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

pub fn router(state: &AppState) -> OpenApiRouter {
    let mut mutating_router = crudrouter(&state.db.clone());

    // Add custom routes for fetching related data with OpenAPI documentation
    mutating_router = mutating_router
        .route(
            "/{id}/samples",
            get(get_location_samples).with_state(state.clone()),
        )
        .route(
            "/{id}/experiments",
            get(get_location_experiments).with_state(state.clone()),
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
            Location::RESOURCE_NAME_PLURAL
        );
    }

    mutating_router
}

/// Get all samples for a specific location
/// Returns lightweight sample data with treatments included
#[utoipa::path(
    get,
    path = "/locations/{id}/samples",
    params(
        ("id" = Uuid, Path, description = "Location ID to fetch samples for")
    ),
    responses(
        (status = 200, description = "List of samples for this location", body = Vec<crate::samples::models::Sample>),
        (status = 404, description = "Location not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "locations",
    summary = "Get location samples",
    description = "Retrieve all samples associated with a specific location, including their treatments"
)]
pub async fn get_location_samples(
    Path(location_id): Path<Uuid>,
    State(app_state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let db = &app_state.db;

    // Get samples for this location with their treatments
    let samples_with_treatments = crate::samples::models::Entity::find()
        .filter(crate::samples::models::Column::LocationId.eq(location_id))
        .find_with_related(crate::treatments::models::Entity)
        .all(db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            )
        })?;

    let mut samples_data = Vec::new();

    for (sample, treatments) in samples_with_treatments {
        // Convert treatments to the format expected by UI
        let treatments_data: Vec<Value> = treatments
            .into_iter()
            .map(|t| {
                json!({
                    "id": t.id,
                    "name": t.name,
                    "notes": t.notes,
                    "enzyme_volume_litres": t.enzyme_volume_litres
                })
            })
            .collect();

        samples_data.push(json!({
            "id": sample.id,
            "name": sample.name,
            "type": sample.r#type,
            "start_time": sample.start_time,
            "stop_time": sample.stop_time,
            "flow_litres_per_minute": sample.flow_litres_per_minute,
            "total_volume": sample.total_volume,
            "material_description": sample.material_description,
            "extraction_procedure": sample.extraction_procedure,
            "filter_substrate": sample.filter_substrate,
            "suspension_volume_litres": sample.suspension_volume_litres,
            "air_volume_litres": sample.air_volume_litres,
              "initial_concentration_gram_l": sample.initial_concentration_gram_l,
            "well_volume_litres": sample.well_volume_litres,
            "remarks": sample.remarks,
            "longitude": sample.longitude,
            "latitude": sample.latitude,
            "location_id": sample.location_id,
            "created_at": sample.created_at,
            "last_updated": sample.last_updated,
            "treatments": treatments_data
        }));
    }

    Ok(Json(json!(samples_data)))
}

/// Get all experiments for a specific location
/// Returns experiments related to this location via samples -> treatments -> regions -> experiments
#[utoipa::path(
    get,
    path = "/locations/{id}/experiments",
    params(
        ("id" = Uuid, Path, description = "Location ID to fetch experiments for")
    ),
    responses(
        (status = 200, description = "List of experiments for this location", body = Vec<crate::experiments::models::Experiment>),
        (status = 404, description = "Location not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "locations",
    summary = "Get location experiments",
    description = "Retrieve all experiments related to a specific location via the relationship chain: location -> samples -> treatments -> regions -> experiments"
)]
pub async fn get_location_experiments(
    Path(location_id): Path<Uuid>,
    State(app_state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let db = &app_state.db;

    // Query experiments related to this location via the relationship chain:
    // location -> samples -> treatments -> regions -> experiments
    let experiments_query = r"
        SELECT DISTINCT e.id, e.name, e.username, e.performed_at,
                       e.temperature_ramp, e.temperature_start, e.temperature_end,
                       e.is_calibration, e.remarks, e.tray_configuration_id,
                       e.created_at, e.last_updated
        FROM experiments e
        JOIN regions r ON r.experiment_id = e.id
        JOIN treatments t ON t.id = r.treatment_id
        JOIN samples s ON s.id = t.sample_id
        WHERE s.location_id = $1
        ORDER BY e.performed_at DESC
    ";

    let experiments: Vec<crate::experiments::models::Model> =
        crate::experiments::models::Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                db.get_database_backend(),
                experiments_query,
                vec![location_id.into()],
            ))
            .all(db)
            .await
            .map_err(|e| {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {e}"),
                )
            })?;

    // Convert to JSON format expected by UI
    let experiments_data: Vec<Value> = experiments
        .into_iter()
        .map(|e| {
            json!({
                "id": e.id,
                "name": e.name,
                "username": e.username,
                "performed_at": e.performed_at,
                "temperature_ramp": e.temperature_ramp,
                "temperature_start": e.temperature_start,
                "temperature_end": e.temperature_end,
                "is_calibration": e.is_calibration,
                "remarks": e.remarks,
                "tray_configuration_id": e.tray_configuration_id,
                "created_at": e.created_at,
                "last_updated": e.last_updated
            })
        })
        .collect();

    Ok(Json(json!(experiments_data)))
}
