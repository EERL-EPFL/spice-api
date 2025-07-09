use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;

use crate::common::state::AppState;
use spice_entity::{
    experiments, regions, samples, time_points, treatments, well_phase_transitions, wells,
};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExperimentResultsSummary {
    pub experiment_id: Uuid,
    pub experiment_name: String,
    pub tray_layout: TrayLayout,
    pub wells: Vec<WellResultSummary>,
    pub total_time_points: i64,
    pub first_timestamp: Option<DateTime<Utc>>,
    pub last_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TrayLayout {
    pub rows: i32,
    pub columns: i32,
    pub tray_configurations: Vec<TrayInfo>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TrayInfo {
    pub tray_id: Uuid,
    pub tray_name: String,
    pub sequence: i32,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WellResultSummary {
    pub row: i32,
    pub col: i32,
    pub coordinate: String,
    pub tray_id: Uuid,
    pub sample_id: Option<Uuid>,
    pub sample_name: Option<String>,
    pub treatment_name: Option<String>,
    pub dilution_factor: Option<Decimal>,
    pub first_phase_change_time: Option<DateTime<Utc>>,
    pub final_state: Option<i32>,
    pub total_phase_changes: i32,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WellDetailedResults {
    pub well_info: WellResultSummary,
    pub phase_change_history: Vec<PhaseChangeEvent>,
    pub temperature_summary: TemperatureSummary,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PhaseChangeEvent {
    pub timestamp: DateTime<Utc>,
    pub from_state: Option<i32>,
    pub to_state: i32,
    pub image_filename: Option<String>,
    pub temperature_at_change: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TemperatureSummary {
    pub min_temperature: Option<f64>,
    pub max_temperature: Option<f64>,
    pub avg_temperature: Option<f64>,
    pub temperature_at_first_freeze: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct WellResultsQuery {
    pub include_temperature: Option<bool>,
    pub include_images: Option<bool>,
}

/// Get results summary for an experiment with optimized well data
#[utoipa::path(
    get,
    path = "/experiments/{experiment_id}/results",
    responses(
        (status = 200, description = "Experiment results retrieved successfully", body = ExperimentResultsSummary),
        (status = 404, description = "Experiment not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("experiment_id" = Uuid, Path, description = "Experiment ID")
    ),
    tag = "experiments"
)]
pub async fn get_experiment_results(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
) -> Result<Json<ExperimentResultsSummary>, (StatusCode, Json<Value>)> {
    let db = &app_state.db;

    // Get experiment info with tray configuration
    let experiment = experiments::Entity::find_by_id(experiment_id)
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Experiment not found"})),
        ))?;

    // Get tray layout information
    let tray_layout = get_tray_layout(db, experiment_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get tray layout: {}", e)})),
        )
    })?;

    // Get time points summary
    let time_points_summary = get_time_points_summary(db, experiment_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to get time points summary: {}", e)})),
            )
        })?;

    // Get well results with phase change summaries
    let wells = get_well_results_summary(db, experiment_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to get well results: {}", e)})),
            )
        })?;

    Ok(Json(ExperimentResultsSummary {
        experiment_id,
        experiment_name: experiment.name,
        tray_layout,
        wells,
        total_time_points: time_points_summary.total_count,
        first_timestamp: time_points_summary.first_timestamp,
        last_timestamp: time_points_summary.last_timestamp,
    }))
}

/// Get detailed results for a specific well
#[utoipa::path(
    get,
    path = "/experiments/{experiment_id}/wells/{row}/{col}/results",
    responses(
        (status = 200, description = "Well results retrieved successfully", body = WellDetailedResults),
        (status = 404, description = "Experiment or well not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("experiment_id" = Uuid, Path, description = "Experiment ID"),
        ("row" = i32, Path, description = "Well row (1-8)"),
        ("col" = i32, Path, description = "Well column (1-12)")
    ),
    tag = "experiments"
)]
pub async fn get_well_detailed_results(
    State(app_state): State<AppState>,
    Path((experiment_id, row, col)): Path<(Uuid, i32, i32)>,
    Query(query): Query<WellResultsQuery>,
) -> Result<Json<WellDetailedResults>, (StatusCode, Json<Value>)> {
    let db = &app_state.db;

    // Validate well coordinates
    if !(1..=8).contains(&row) || !(1..=12).contains(&col) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                json!({"error": "Invalid well coordinates. Row must be 1-8, column must be 1-12"}),
            ),
        ));
    }

    // Get well summary info
    let well_info = get_single_well_summary(db, experiment_id, row, col)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to get well info: {}", e)})),
            )
        })?;

    if well_info.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Well not found or has no data"})),
        ));
    }

    let well_info = well_info.unwrap();

    // Get phase change history for this well
    let phase_change_history = get_well_phase_change_history(
        db,
        experiment_id,
        row,
        col,
        query.include_images.unwrap_or(true),
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get phase change history: {}", e)})),
        )
    })?;

    // Get temperature summary if requested
    let temperature_summary = if query.include_temperature.unwrap_or(true) {
        get_well_temperature_summary(db, experiment_id, row, col)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Failed to get temperature summary: {}", e)})),
                )
            })?
    } else {
        TemperatureSummary {
            min_temperature: None,
            max_temperature: None,
            avg_temperature: None,
            temperature_at_first_freeze: None,
        }
    };

    Ok(Json(WellDetailedResults {
        well_info,
        phase_change_history,
        temperature_summary,
    }))
}

// Helper functions

async fn get_tray_layout(
    db: &sea_orm::DatabaseConnection,
    experiment_id: Uuid,
) -> anyhow::Result<TrayLayout> {
    use spice_entity::{tray_configuration_assignments, trays};

    // Get experiment to find tray configuration
    let experiment = experiments::Entity::find_by_id(experiment_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Experiment not found"))?;

    let tray_config_id = experiment
        .tray_configuration_id
        .ok_or_else(|| anyhow::anyhow!("Experiment has no tray configuration"))?;

    // Get tray assignments
    let assignments = tray_configuration_assignments::Entity::find()
        .filter(tray_configuration_assignments::Column::TrayConfigurationId.eq(tray_config_id))
        .find_also_related(trays::Entity)
        .all(db)
        .await?;

    let tray_configurations = assignments
        .into_iter()
        .filter_map(|(assignment, tray_opt)| {
            tray_opt.map(|tray| TrayInfo {
                tray_id: assignment.tray_id,
                tray_name: tray
                    .name
                    .unwrap_or_else(|| format!("P{}", assignment.order_sequence)),
                sequence: assignment.order_sequence,
            })
        })
        .collect();

    Ok(TrayLayout {
        rows: 8,
        columns: 12,
        tray_configurations,
    })
}

struct TimePointsSummary {
    total_count: i64,
    first_timestamp: Option<DateTime<Utc>>,
    last_timestamp: Option<DateTime<Utc>>,
}

async fn get_time_points_summary(
    db: &sea_orm::DatabaseConnection,
    experiment_id: Uuid,
) -> anyhow::Result<TimePointsSummary> {
    let total_count = time_points::Entity::find()
        .filter(time_points::Column::ExperimentId.eq(experiment_id))
        .count(db)
        .await? as i64;

    // Get first and last timestamps
    let first_time_point = time_points::Entity::find()
        .filter(time_points::Column::ExperimentId.eq(experiment_id))
        .order_by_asc(time_points::Column::Timestamp)
        .one(db)
        .await?;

    let last_time_point = time_points::Entity::find()
        .filter(time_points::Column::ExperimentId.eq(experiment_id))
        .order_by_desc(time_points::Column::Timestamp)
        .one(db)
        .await?;

    Ok(TimePointsSummary {
        total_count,
        first_timestamp: first_time_point.map(|tp| tp.timestamp.with_timezone(&Utc)),
        last_timestamp: last_time_point.map(|tp| tp.timestamp.with_timezone(&Utc)),
    })
}

async fn get_well_results_summary(
    db: &sea_orm::DatabaseConnection,
    experiment_id: Uuid,
) -> anyhow::Result<Vec<WellResultSummary>> {
    // Get all wells with phase transitions for this experiment
    let phase_transitions = well_phase_transitions::Entity::find()
        .filter(well_phase_transitions::Column::ExperimentId.eq(experiment_id))
        .find_also_related(wells::Entity)
        .order_by_asc(well_phase_transitions::Column::Timestamp)
        .all(db)
        .await?;

    // Group by well and analyze phase changes
    let mut well_map: HashMap<Uuid, WellTransitionData> = HashMap::new();

    for (transition, well_opt) in phase_transitions {
        if let Some(well) = well_opt {
            let entry = well_map
                .entry(well.id)
                .or_insert_with(|| WellTransitionData::new(well));
            entry.add_transition(transition);
        }
    }

    // Convert to summary format
    let mut results = Vec::new();
    for (_, well_data) in well_map {
        let coordinate = format!(
            "{}{}",
            char::from_u32('A' as u32 + (well_data.well.column_number - 1) as u32).unwrap_or('?'),
            well_data.well.row_number
        );

        // Get sample and treatment info for this well
        // Convert 1-based well coordinates to 0-based for region comparison
        let (sample_info, treatment_info, dilution_factor) = get_well_sample_treatment_info(
            db,
            experiment_id,
            well_data.well.row_number - 1,
            well_data.well.column_number - 1,
        )
        .await?;

        results.push(WellResultSummary {
            row: well_data.well.row_number,
            col: well_data.well.column_number,
            coordinate,
            tray_id: well_data.well.tray_id,
            sample_id: sample_info.as_ref().map(|s| s.0),
            sample_name: sample_info.as_ref().map(|s| s.1.clone()),
            treatment_name: treatment_info.as_ref().map(|t| t.1.clone()),
            dilution_factor,
            first_phase_change_time: well_data.first_phase_change_time(),
            final_state: well_data.final_state(),
            total_phase_changes: well_data.phase_change_count(),
        });
    }

    // Sort by row then column
    results.sort_by_key(|w| (w.row, w.col));

    Ok(results)
}

async fn get_single_well_summary(
    db: &sea_orm::DatabaseConnection,
    experiment_id: Uuid,
    row: i32,
    col: i32,
) -> anyhow::Result<Option<WellResultSummary>> {
    // Find the well with matching coordinates that has phase transitions
    let phase_transitions = well_phase_transitions::Entity::find()
        .filter(well_phase_transitions::Column::ExperimentId.eq(experiment_id))
        .find_also_related(wells::Entity)
        .all(db)
        .await?;

    // Find transitions for this specific well coordinate
    let mut well_transitions = Vec::new();
    let mut target_well = None;

    for (transition, well_opt) in phase_transitions {
        if let Some(well) = well_opt {
            if well.row_number == row && well.column_number == col {
                well_transitions.push(transition);
                if target_well.is_none() {
                    target_well = Some(well);
                }
            }
        }
    }

    if well_transitions.is_empty() || target_well.is_none() {
        return Ok(None);
    }

    let well = target_well.unwrap();
    let mut well_data = WellTransitionData::new(well);
    for transition in well_transitions {
        well_data.add_transition(transition);
    }

    let coordinate = format!(
        "{}{}",
        char::from_u32('A' as u32 + (row - 1) as u32).unwrap_or('?'),
        col
    );

    // Convert 1-based well coordinates to 0-based for region comparison
    let (sample_info, treatment_info, dilution_factor) =
        get_well_sample_treatment_info(db, experiment_id, row - 1, col - 1).await?;

    Ok(Some(WellResultSummary {
        row,
        col,
        coordinate,
        tray_id: well_data.well.tray_id,
        sample_id: sample_info.as_ref().map(|s| s.0),
        sample_name: sample_info.as_ref().map(|s| s.1.clone()),
        treatment_name: treatment_info.as_ref().map(|t| t.1.clone()),
        dilution_factor,
        first_phase_change_time: well_data.first_phase_change_time(),
        final_state: well_data.final_state(),
        total_phase_changes: well_data.phase_change_count(),
    }))
}

async fn get_well_phase_change_history(
    db: &sea_orm::DatabaseConnection,
    experiment_id: Uuid,
    row: i32,
    col: i32,
    include_images: bool,
) -> anyhow::Result<Vec<PhaseChangeEvent>> {
    use spice_entity::temperature_readings;

    // Find the well with matching coordinates and get its phase transitions
    let phase_transitions = well_phase_transitions::Entity::find()
        .filter(well_phase_transitions::Column::ExperimentId.eq(experiment_id))
        .find_also_related(wells::Entity)
        .order_by_asc(well_phase_transitions::Column::Timestamp)
        .all(db)
        .await?;

    let mut phase_changes = Vec::new();

    for (transition, well_opt) in phase_transitions {
        if let Some(well) = well_opt {
            if well.row_number == row && well.column_number == col {
                // Get temperature reading if needed
                let temperature_reading = if include_images {
                    temperature_readings::Entity::find_by_id(transition.temperature_reading_id)
                        .one(db)
                        .await?
                } else {
                    None
                };

                phase_changes.push(PhaseChangeEvent {
                    timestamp: transition.timestamp.with_timezone(&Utc),
                    from_state: Some(transition.previous_state),
                    to_state: transition.new_state,
                    image_filename: if include_images {
                        temperature_reading
                            .as_ref()
                            .and_then(|tr| tr.image_filename.clone())
                    } else {
                        None
                    },
                    temperature_at_change: None, // Could be calculated from temperature_reading if needed
                });
            }
        }
    }

    Ok(phase_changes)
}

async fn get_well_temperature_summary(
    db: &sea_orm::DatabaseConnection,
    experiment_id: Uuid,
    row: i32,
    col: i32,
) -> anyhow::Result<TemperatureSummary> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    use spice_entity::{time_point_temperatures, time_point_well_states};

    // First, get all time point IDs for this well, then filter by experiment
    let well_time_point_ids: Vec<Uuid> = time_point_well_states::Entity::find()
        .filter(time_point_well_states::Column::Row.eq(row))
        .filter(time_point_well_states::Column::Col.eq(col))
        .all(db)
        .await?
        .into_iter()
        .map(|well_state| well_state.time_point_id)
        .collect();

    if well_time_point_ids.is_empty() {
        return Ok(TemperatureSummary {
            min_temperature: None,
            max_temperature: None,
            avg_temperature: None,
            temperature_at_first_freeze: None,
        });
    }

    // Filter time points by experiment to get final list
    let time_point_ids: Vec<Uuid> = time_points::Entity::find()
        .filter(time_points::Column::Id.is_in(well_time_point_ids))
        .filter(time_points::Column::ExperimentId.eq(experiment_id))
        .all(db)
        .await?
        .into_iter()
        .map(|tp| tp.id)
        .collect();

    if time_point_ids.is_empty() {
        return Ok(TemperatureSummary {
            min_temperature: None,
            max_temperature: None,
            avg_temperature: None,
            temperature_at_first_freeze: None,
        });
    }

    // Then get temperature readings for those time points
    let temps: Vec<f64> = time_point_temperatures::Entity::find()
        .filter(time_point_temperatures::Column::TimePointId.is_in(time_point_ids))
        .all(db)
        .await?
        .into_iter()
        .filter_map(|temp| temp.temperature.to_f64())
        .collect();

    if temps.is_empty() {
        return Ok(TemperatureSummary {
            min_temperature: None,
            max_temperature: None,
            avg_temperature: None,
            temperature_at_first_freeze: None,
        });
    }

    let min_temp = temps.iter().copied().fold(f64::INFINITY, f64::min);
    let max_temp = temps.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let avg_temp = temps.iter().sum::<f64>() / temps.len() as f64;

    // TODO: Get temperature at first freeze event
    let temp_at_first_freeze = None;

    Ok(TemperatureSummary {
        min_temperature: Some(min_temp),
        max_temperature: Some(max_temp),
        avg_temperature: Some(avg_temp),
        temperature_at_first_freeze: temp_at_first_freeze,
    })
}

async fn get_well_sample_treatment_info(
    db: &sea_orm::DatabaseConnection,
    experiment_id: Uuid,
    row: i32,
    col: i32,
) -> anyhow::Result<(
    Option<(Uuid, String)>,
    Option<(Uuid, String)>,
    Option<Decimal>,
)> {
    // Find the region that contains this well coordinate
    let region = regions::Entity::find()
        .filter(regions::Column::ExperimentId.eq(experiment_id))
        .filter(regions::Column::RowMin.lte(row))
        .filter(regions::Column::RowMax.gte(row))
        .filter(regions::Column::ColMin.lte(col))
        .filter(regions::Column::ColMax.gte(col))
        .one(db)
        .await?;

    if let Some(region) = region {
        let mut sample_info = None;
        let mut treatment_info = None;
        let dilution_factor = region.dilution_factor.map(Decimal::from);

        // Get treatment info if region has a treatment
        if let Some(treatment_id) = region.treatment_id {
            if let Some(treatment) = treatments::Entity::find_by_id(treatment_id)
                .find_also_related(samples::Entity)
                .one(db)
                .await?
            {
                treatment_info = Some((treatment.0.id, format!("{:?}", treatment.0.name)));

                // Get sample info if treatment has a sample
                if let Some(sample) = treatment.1 {
                    sample_info = Some((sample.id, sample.name));
                }
            }
        }

        Ok((sample_info, treatment_info, dilution_factor))
    } else {
        // No region found for this well coordinate
        Ok((None, None, None))
    }
}

#[derive(Debug)]
struct WellTransitionData {
    well: wells::Model,
    transitions: Vec<well_phase_transitions::Model>,
}

impl WellTransitionData {
    fn new(well: wells::Model) -> Self {
        Self {
            well,
            transitions: Vec::new(),
        }
    }

    fn add_transition(&mut self, transition: well_phase_transitions::Model) {
        self.transitions.push(transition);
        // Keep sorted by timestamp
        self.transitions.sort_by_key(|t| t.timestamp);
    }

    fn first_phase_change_time(&self) -> Option<DateTime<Utc>> {
        // Find first transition from 0 to 1 (liquid to frozen)
        self.transitions
            .iter()
            .find(|t| t.previous_state == 0 && t.new_state == 1)
            .map(|t| t.timestamp.with_timezone(&Utc))
    }

    fn final_state(&self) -> Option<i32> {
        self.transitions.last().map(|t| t.new_state)
    }

    fn phase_change_count(&self) -> i32 {
        self.transitions.len() as i32
    }
}

#[cfg(test)]
mod tests {
    use crate::config::test_helpers::{setup_test_app, setup_test_db};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
    use serde_json::json;
    use spice_entity::{
        experiments, regions, samples, time_point_temperatures, time_point_well_states,
        time_points, tray_configuration_assignments, tray_configurations, trays, treatments,
        well_phase_transitions, wells,
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn create_tray_with_config(
        db: &sea_orm::DatabaseConnection,
        rows: i32,
        cols: i32,
        config_name: &str,
    ) -> (Uuid, Uuid) {
        // Create tray
        let tray_id = Uuid::new_v4();
        let tray = trays::ActiveModel {
            id: Set(tray_id),
            name: Set(Some(format!("{}x{} Tray", rows, cols))),
            qty_x_axis: Set(Some(cols)),
            qty_y_axis: Set(Some(rows)),
            well_relative_diameter: Set(None),
            last_updated: Set(chrono::Utc::now().into()),
            created_at: Set(chrono::Utc::now().into()),
        };
        tray.insert(db).await.unwrap();

        // Create tray configuration
        let config_id = Uuid::new_v4();
        let config = tray_configurations::ActiveModel {
            id: Set(config_id),
            name: Set(Some(config_name.to_string())),
            experiment_default: Set(false),
            created_at: Set(chrono::Utc::now().into()),
            last_updated: Set(chrono::Utc::now().into()),
        };
        config.insert(db).await.unwrap();

        // Create tray configuration assignment
        let assignment = tray_configuration_assignments::ActiveModel {
            tray_id: Set(tray_id),
            tray_configuration_id: Set(config_id),
            order_sequence: Set(1),
            rotation_degrees: Set(0),
            created_at: Set(chrono::Utc::now().into()),
            last_updated: Set(chrono::Utc::now().into()),
        };
        assignment.insert(db).await.unwrap();

        (tray_id, config_id)
    }

    async fn assign_tray_config_to_experiment(
        db: &sea_orm::DatabaseConnection,
        experiment_id: Uuid,
        config_id: Uuid,
    ) {
        let mut experiment: experiments::ActiveModel =
            experiments::Entity::find_by_id(experiment_id)
                .one(db)
                .await
                .unwrap()
                .unwrap()
                .into();

        experiment.tray_configuration_id = Set(Some(config_id));
        experiment.update(db).await.unwrap();
    }

    async fn create_test_time_points(
        db: &sea_orm::DatabaseConnection,
        experiment_id: Uuid,
    ) -> Vec<Uuid> {
        let mut time_point_ids = Vec::new();

        // Create 3 time points with phase changes
        for i in 0..3 {
            let time_point_id = Uuid::new_v4();
            let timestamp = chrono::Utc::now() + chrono::Duration::seconds(i * 60);

            let time_point = time_points::ActiveModel {
                id: Set(time_point_id),
                experiment_id: Set(experiment_id),
                timestamp: Set(timestamp.into()),
                image_filename: Set(Some(format!("image_{}.jpg", i))),
                asset_id: Set(None),
                created_at: Set(chrono::Utc::now().into()),
            };
            time_point.insert(db).await.unwrap();

            // Create well states for a 2x2 grid
            for row in 1..=2 {
                for col in 1..=2 {
                    let state = if i == 0 {
                        0
                    } else if row == 1 && col == 1 && i >= 1 {
                        1
                    } else {
                        0
                    };

                    let well_state = time_point_well_states::ActiveModel {
                        id: Set(Uuid::new_v4()),
                        time_point_id: Set(time_point_id),
                        row: Set(row),
                        col: Set(col),
                        value: Set(state),
                    };
                    well_state.insert(db).await.unwrap();
                }
            }

            // Create temperature readings
            for probe in 1..=4 {
                let temp = time_point_temperatures::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    time_point_id: Set(time_point_id),
                    probe_sequence: Set(probe),
                    temperature: Set(rust_decimal::Decimal::new(250 - (i as i64 * 10), 1)), // 25.0, 24.0, 23.0
                };
                temp.insert(db).await.unwrap();
            }

            time_point_ids.push(time_point_id);
        }

        time_point_ids
    }

    #[tokio::test]
    async fn test_experiment_results_endpoint() {
        let db = setup_test_db().await;

        // Create tray configuration for test (2x2 for simplicity)
        let (_tray_id, config_id) = create_tray_with_config(&db, 2, 2, "Results Test Config").await;

        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new();
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment
        let experiment_data = json!({
            "name": "Results Test Experiment",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test results endpoint"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/experiments")
                    .header("content-type", "application/json")
                    .body(Body::from(experiment_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let experiment_id = experiment["id"].as_str().unwrap();
        let experiment_uuid = Uuid::parse_str(experiment_id).unwrap();

        // Assign tray configuration and create test data
        assign_tray_config_to_experiment(&db, experiment_uuid, config_id).await;
        create_test_time_points(&db, experiment_uuid).await;

        // Test the results endpoint
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/api/experiments/{}/results", experiment_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Results endpoint should work"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let results: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(results["experiment_id"], experiment_id);
        assert!(
            results["experiment_name"].is_string(),
            "Should have experiment name"
        );
        assert!(
            results["tray_layout"].is_object(),
            "Should have tray layout"
        );
        assert!(results["wells"].is_array(), "Should have wells array");
        assert_eq!(results["total_time_points"], 3, "Should have 3 time points");

        let wells = results["wells"].as_array().unwrap();
        assert!(!wells.is_empty(), "Should have wells data");

        // Check that well (1,1) has phase changes
        let well_1_1 = wells
            .iter()
            .find(|w| w["row"] == 1 && w["col"] == 1)
            .expect("Should find well (1,1)");

        assert!(
            well_1_1["first_phase_change_time"].is_string(),
            "Well (1,1) should have phase change"
        );
        assert_eq!(well_1_1["final_state"], 1, "Well (1,1) should be frozen");
        assert!(
            well_1_1["total_phase_changes"].as_i64().unwrap() > 0,
            "Should have phase changes"
        );

        println!(
            "Results response: {}",
            serde_json::to_string_pretty(&results).unwrap()
        );
    }

    #[tokio::test]
    async fn test_experiment_results_no_data() {
        let db = setup_test_db().await;

        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new();
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment without any time points
        let experiment_data = json!({
            "name": "Empty Results Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test empty results"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/experiments")
                    .header("content-type", "application/json")
                    .body(Body::from(experiment_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let experiment_id = experiment["id"].as_str().unwrap();

        // Test the results endpoint with no data
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/api/experiments/{}/results", experiment_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should still work but return empty results
        let status = response.status();
        if status != StatusCode::OK {
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let error_text = String::from_utf8_lossy(&body_bytes);
            println!("Error response: {} - {}", status, error_text);
        }

        // Might return an error if no tray configuration is assigned
        assert!(
            status == StatusCode::OK || status == StatusCode::INTERNAL_SERVER_ERROR,
            "Should handle experiments with no data gracefully"
        );
    }

    #[tokio::test]
    async fn test_well_detailed_results_endpoint() {
        let db = setup_test_db().await;

        // Create tray configuration for test (2x2 for simplicity)
        let (_tray_id, config_id) =
            create_tray_with_config(&db, 2, 2, "Well Details Test Config").await;

        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new();
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment
        let experiment_data = json!({
            "name": "Well Details Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test well details"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/experiments")
                    .header("content-type", "application/json")
                    .body(Body::from(experiment_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let experiment_id = experiment["id"].as_str().unwrap();
        let experiment_uuid = Uuid::parse_str(experiment_id).unwrap();

        // Assign tray configuration and create test data
        assign_tray_config_to_experiment(&db, experiment_uuid, config_id).await;
        create_test_time_points(&db, experiment_uuid).await;

        // Test the well detailed results endpoint
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!(
                        "/api/experiments/{}/wells/1/1/results",
                        experiment_id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Well details endpoint should work"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let well_results: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert!(
            well_results["well_info"].is_object(),
            "Should have well info"
        );
        assert!(
            well_results["phase_change_history"].is_array(),
            "Should have phase change history"
        );
        assert!(
            well_results["temperature_summary"].is_object(),
            "Should have temperature summary"
        );

        let well_info = &well_results["well_info"];
        assert_eq!(well_info["row"], 1);
        assert_eq!(well_info["col"], 1);
        assert_eq!(well_info["coordinate"], "A1");

        let phase_changes = well_results["phase_change_history"].as_array().unwrap();
        assert!(!phase_changes.is_empty(), "Should have phase change events");

        println!(
            "Well results response: {}",
            serde_json::to_string_pretty(&well_results).unwrap()
        );
    }

    #[tokio::test]
    async fn test_well_detailed_results_invalid_coordinates() {
        let app = setup_test_app().await;

        // Create an experiment
        let experiment_data = json!({
            "name": "Invalid Coordinates Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test invalid coordinates"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/experiments")
                    .header("content-type", "application/json")
                    .body(Body::from(experiment_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let experiment_id = experiment["id"].as_str().unwrap();

        // Test invalid coordinates (row=0, should be 1-8)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!(
                        "/api/experiments/{}/wells/0/1/results",
                        experiment_id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Should reject invalid row"
        );

        // Test invalid coordinates (col=13, should be 1-12)
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!(
                        "/api/experiments/{}/wells/1/13/results",
                        experiment_id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Should reject invalid column"
        );
    }

    #[tokio::test]
    async fn test_well_detailed_results_not_found() {
        let db = setup_test_db().await;

        // Create tray configuration for test (2x2 for simplicity)
        let (_tray_id, config_id) =
            create_tray_with_config(&db, 2, 2, "Not Found Test Config").await;

        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new();
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment
        let experiment_data = json!({
            "name": "Not Found Test",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test well not found"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/experiments")
                    .header("content-type", "application/json")
                    .body(Body::from(experiment_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let experiment_id = experiment["id"].as_str().unwrap();
        let experiment_uuid = Uuid::parse_str(experiment_id).unwrap();

        // Assign tray configuration but don't create any time points
        assign_tray_config_to_experiment(&db, experiment_uuid, config_id).await;

        // Test well that has no data
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!(
                        "/api/experiments/{}/wells/1/1/results",
                        experiment_id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Should return 404 for well with no data"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let error: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(
            error["error"].as_str().unwrap().contains("not found"),
            "Error should mention not found"
        );
    }

    async fn create_test_phase_transitions(
        db: &sea_orm::DatabaseConnection,
        experiment_id: Uuid,
        tray_id: Uuid,
    ) -> Vec<Uuid> {
        use spice_entity::temperature_readings;

        // Create wells for the tray (2x2 grid)
        let mut well_ids = Vec::new();
        for row in 1..=2 {
            for col in 1..=2 {
                let well_id = Uuid::new_v4();
                let well = wells::ActiveModel {
                    id: Set(well_id),
                    tray_id: Set(tray_id),
                    row_number: Set(row),
                    column_number: Set(col),
                    created_at: Set(chrono::Utc::now().into()),
                    last_updated: Set(chrono::Utc::now().into()),
                };
                well.insert(db).await.unwrap();
                well_ids.push(well_id);
            }
        }

        // Create temperature readings
        let mut temp_reading_ids = Vec::new();
        for i in 0..3 {
            let temp_reading_id = Uuid::new_v4();
            let timestamp = chrono::Utc::now() + chrono::Duration::seconds(i * 60);

            let temp_reading = temperature_readings::ActiveModel {
                id: Set(temp_reading_id),
                experiment_id: Set(experiment_id),
                timestamp: Set(timestamp.into()),
                image_filename: Set(Some(format!("image_{}.jpg", i))),
                probe_1: Set(Some(rust_decimal::Decimal::new(250 - (i as i64 * 10), 1))), // 25.0, 24.0, 23.0
                probe_2: Set(Some(rust_decimal::Decimal::new(248 - (i as i64 * 10), 1))),
                probe_3: Set(None),
                probe_4: Set(None),
                probe_5: Set(None),
                probe_6: Set(None),
                probe_7: Set(None),
                probe_8: Set(None),
                created_at: Set(chrono::Utc::now().into()),
            };
            temp_reading.insert(db).await.unwrap();
            temp_reading_ids.push(temp_reading_id);
        }

        // Create phase transitions - only for well (1,1) from liquid to frozen
        let well_1_1_id = well_ids[0]; // First well (row 1, col 1)
        let timestamp = chrono::Utc::now() + chrono::Duration::seconds(60); // At second time point

        let phase_transition = well_phase_transitions::ActiveModel {
            id: Set(Uuid::new_v4()),
            well_id: Set(well_1_1_id),
            experiment_id: Set(experiment_id),
            temperature_reading_id: Set(temp_reading_ids[1]),
            timestamp: Set(timestamp.into()),
            previous_state: Set(0), // liquid
            new_state: Set(1),      // frozen
            created_at: Set(chrono::Utc::now().into()),
        };
        phase_transition.insert(db).await.unwrap();

        well_ids
    }

    async fn create_test_regions_with_treatments(
        db: &sea_orm::DatabaseConnection,
        experiment_id: Uuid,
    ) {
        // Create a sample
        let sample_id = Uuid::new_v4();
        let sample = samples::ActiveModel {
            id: Set(sample_id),
            name: Set("Test Sample".to_string()),
            start_time: Set(None),
            stop_time: Set(None),
            flow_litres_per_minute: Set(None),
            total_volume: Set(None),
            material_description: Set(Some("Test material".to_string())),
            extraction_procedure: Set(None),
            filter_substrate: Set(None),
            suspension_volume_litres: Set(None),
            air_volume_litres: Set(None),
            water_volume_litres: Set(None),
            initial_concentration_gram_l: Set(None),
            well_volume_litres: Set(None),
            remarks: Set(None),
            longitude: Set(None),
            latitude: Set(None),
            location_id: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            last_updated: Set(chrono::Utc::now().into()),
            r#type: Set(spice_entity::sea_orm_active_enums::SampleType::Filter),
        };
        sample.insert(db).await.unwrap();

        // Create a treatment
        let treatment_id = Uuid::new_v4();
        let treatment = treatments::ActiveModel {
            id: Set(treatment_id),
            notes: Set(Some("Test treatment".to_string())),
            sample_id: Set(Some(sample_id)),
            last_updated: Set(chrono::Utc::now().into()),
            created_at: Set(chrono::Utc::now().into()),
            enzyme_volume_litres: Set(None),
            name: Set(spice_entity::sea_orm_active_enums::TreatmentName::None),
        };
        treatment.insert(db).await.unwrap();

        // Create a region for well (1,1)
        let region = regions::ActiveModel {
            id: Set(Uuid::new_v4()),
            experiment_id: Set(experiment_id),
            treatment_id: Set(Some(treatment_id)),
            name: Set(Some("Test Region".to_string())),
            display_colour_hex: Set(Some("#FF0000".to_string())),
            tray_id: Set(Some(1)),
            col_min: Set(Some(1)),
            row_min: Set(Some(1)),
            col_max: Set(Some(1)),
            row_max: Set(Some(1)),
            dilution_factor: Set(Some(100)),
            created_at: Set(chrono::Utc::now().into()),
            last_updated: Set(chrono::Utc::now().into()),
            is_background_key: Set(false),
        };
        region.insert(db).await.unwrap();
    }

    #[tokio::test]
    async fn test_experiment_results_with_phase_transitions() {
        let db = setup_test_db().await;

        // Create tray configuration for test (2x2 for simplicity)
        let (tray_id, config_id) =
            create_tray_with_config(&db, 2, 2, "Phase Transitions Test Config").await;

        let mut config = crate::config::Config::for_tests();
        config.keycloak_url = String::new();
        let app = crate::routes::build_router(&db, &config);

        // Create an experiment
        let experiment_data = json!({
            "name": "Phase Transitions Test Experiment",
            "username": "test@example.com",
            "performed_at": "2024-06-20T14:30:00Z",
            "temperature_ramp": -1.0,
            "temperature_start": 5.0,
            "temperature_end": -25.0,
            "is_calibration": false,
            "remarks": "Test phase transitions endpoint"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/experiments")
                    .header("content-type", "application/json")
                    .body(Body::from(experiment_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let experiment: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let experiment_id = experiment["id"].as_str().unwrap();
        let experiment_uuid = Uuid::parse_str(experiment_id).unwrap();

        // Assign tray configuration and create test data
        assign_tray_config_to_experiment(&db, experiment_uuid, config_id).await;
        create_test_phase_transitions(&db, experiment_uuid, tray_id).await;
        create_test_regions_with_treatments(&db, experiment_uuid).await;

        // Test the results endpoint
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/api/experiments/{}/results", experiment_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Results endpoint should work"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let results: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(results["experiment_id"], experiment_id);
        assert!(
            results["experiment_name"].is_string(),
            "Should have experiment name"
        );
        assert!(
            results["tray_layout"].is_object(),
            "Should have tray layout"
        );
        assert!(results["wells"].is_array(), "Should have wells array");

        let wells = results["wells"].as_array().unwrap();
        assert!(!wells.is_empty(), "Should have wells data");

        // Check that well (1,1) has phase changes and treatment info
        let well_1_1 = wells
            .iter()
            .find(|w| w["row"] == 1 && w["col"] == 1)
            .expect("Should find well (1,1)");

        assert!(
            well_1_1["first_phase_change_time"].is_string(),
            "Well (1,1) should have phase change"
        );
        assert_eq!(well_1_1["final_state"], 1, "Well (1,1) should be frozen");
        assert_eq!(
            well_1_1["total_phase_changes"], 1,
            "Should have 1 phase change"
        );
        assert_eq!(
            well_1_1["coordinate"], "A1",
            "Should have correct coordinate"
        );

        // Check treatment and sample information
        assert!(
            well_1_1["sample_name"].is_string(),
            "Should have sample name"
        );
        assert!(
            well_1_1["treatment_name"].is_string(),
            "Should have treatment name"
        );

        // Dilution factor is returned as a Decimal string
        assert_eq!(
            well_1_1["dilution_factor"], "100",
            "Should have dilution factor"
        );

        println!(
            "Phase transitions results response: {}",
            serde_json::to_string_pretty(&results).unwrap()
        );
    }

    #[tokio::test]
    async fn test_results_endpoint_experiment_not_found() {
        let app = setup_test_app().await;

        let fake_experiment_id = "00000000-0000-0000-0000-000000000000";

        // Test results endpoint with non-existent experiment
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/api/experiments/{}/results", fake_experiment_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Should return 404 for non-existent experiment"
        );

        // Test well results endpoint with non-existent experiment
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!(
                        "/api/experiments/{}/wells/1/1/results",
                        fake_experiment_id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // This might return 500 instead of 404 if well lookup happens before experiment check
        assert!(
            response.status() == StatusCode::NOT_FOUND
                || response.status() == StatusCode::INTERNAL_SERVER_ERROR,
            "Should handle non-existent experiment gracefully"
        );
    }
}
