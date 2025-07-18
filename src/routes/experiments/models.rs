use crate::routes::trays::services::{WellCoordinate, coordinates_to_str};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel, traits::MergeIntoActiveModel};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    Order, QueryOrder, QuerySelect, TransactionTrait, entity::prelude::*,
};
use serde::{Deserialize, Serialize};
use spice_entity::experiments::Model;
use spice_entity::sea_orm_active_enums::TreatmentName;
use spice_entity::{experiments, tray_configuration_assignments};
use utoipa::ToSchema;
use uuid::Uuid;

// Convert regions input to active models
fn create_region_active_models(
    experiment_id: Uuid,
    regions: Vec<RegionInput>,
    _db: &impl ConnectionTrait,
) -> Vec<spice_entity::regions::ActiveModel> {
    let mut active_models = Vec::new();

    for region in regions {
        let dilution_factor = region.dilution.as_ref().and_then(|s| s.parse::<i32>().ok());

        let active_model = spice_entity::regions::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            experiment_id: ActiveValue::Set(experiment_id),
            treatment_id: ActiveValue::Set(region.treatment_id),
            name: ActiveValue::Set(region.name),
            display_colour_hex: ActiveValue::Set(region.color),
            tray_id: ActiveValue::Set(region.tray_sequence_id), // Use tray_sequence_id from input
            col_min: ActiveValue::Set(region.col_min),
            row_min: ActiveValue::Set(region.row_min),
            col_max: ActiveValue::Set(region.col_max),
            row_max: ActiveValue::Set(region.row_max),
            dilution_factor: ActiveValue::Set(dilution_factor),
            is_background_key: ActiveValue::Set(region.is_background_key.unwrap_or(false)),
            created_at: ActiveValue::Set(chrono::Utc::now().into()),
            last_updated: ActiveValue::Set(chrono::Utc::now().into()),
        };

        active_models.push(active_model);
    }

    active_models
}

// Fetch treatment information with sample and location data
async fn fetch_treatment_info(
    treatment_id: Uuid,
    db: &impl ConnectionTrait,
) -> Result<Option<TreatmentInfo>, DbErr> {
    let treatment = spice_entity::treatments::Entity::find_by_id(treatment_id)
        .one(db)
        .await?;

    if let Some(treatment) = treatment {
        let sample_info = if let Some(sample_id) = treatment.sample_id {
            let sample = spice_entity::samples::Entity::find_by_id(sample_id)
                .one(db)
                .await?;

            if let Some(sample) = sample {
                let location_info = if let Some(location_id) = sample.location_id {
                    let location = spice_entity::locations::Entity::find_by_id(location_id)
                        .one(db)
                        .await?;

                    location.map(|l| LocationInfo {
                        id: l.id,
                        name: l.name,
                    })
                } else {
                    None
                };

                Some(SampleInfo {
                    id: sample.id,
                    name: sample.name,
                    location: location_info,
                })
            } else {
                None
            }
        } else {
            None
        };

        Ok(Some(TreatmentInfo {
            id: treatment.id,
            name: treatment.name,
            notes: treatment.notes,
            enzyme_volume_litres: treatment.enzyme_volume_litres,
            sample: sample_info,
        }))
    } else {
        Ok(None)
    }
}

// Fetch tray information by sequence ID for a given experiment
async fn fetch_tray_info_by_sequence(
    experiment_id: Uuid,
    tray_sequence_id: i32,
    db: &impl ConnectionTrait,
) -> Result<Option<TrayInfo>, DbErr> {
    use spice_entity::{experiments, tray_configuration_assignments, trays};

    // Get the experiment to find its tray configuration
    let experiment = experiments::Entity::find_by_id(experiment_id)
        .one(db)
        .await?;

    if let Some(exp) = experiment {
        if let Some(tray_config_id) = exp.tray_configuration_id {
            // Find the tray assignment with the matching sequence ID
            let assignment = tray_configuration_assignments::Entity::find()
                .filter(
                    tray_configuration_assignments::Column::TrayConfigurationId.eq(tray_config_id),
                )
                .filter(tray_configuration_assignments::Column::OrderSequence.eq(tray_sequence_id))
                .find_also_related(trays::Entity)
                .one(db)
                .await?;

            if let Some((assignment, Some(tray))) = assignment {
                return Ok(Some(TrayInfo {
                    id: tray.id,
                    name: tray.name,
                    sequence_id: assignment.order_sequence,
                    qty_x_axis: tray.qty_x_axis,
                    qty_y_axis: tray.qty_y_axis,
                    well_relative_diameter: tray.well_relative_diameter.map(|d| d.to_string()),
                }));
            }
        }
    }

    Ok(None)
}

// Convert region model back to RegionInput for response
async fn region_model_to_input_with_treatment(
    region: spice_entity::regions::Model,
    db: &impl ConnectionTrait,
) -> Result<RegionInput, DbErr> {
    let treatment_info = if let Some(treatment_id) = region.treatment_id {
        fetch_treatment_info(treatment_id, db).await?
    } else {
        None
    };

    // Get tray information for this region
    let tray_info = if let Some(tray_sequence_id) = region.tray_id {
        fetch_tray_info_by_sequence(region.experiment_id, tray_sequence_id, db).await?
    } else {
        None
    };

    Ok(RegionInput {
        name: region.name,
        tray_sequence_id: region.tray_id, // Map tray_id from DB to tray_sequence_id in response
        col_min: region.col_min,
        col_max: region.col_max,
        row_min: region.row_min,
        row_max: region.row_max,
        color: region.display_colour_hex,
        dilution: region.dilution_factor.map(|d| d.to_string()),
        treatment_id: region.treatment_id,
        is_background_key: Some(region.is_background_key),
        treatment: treatment_info,
        tray: tray_info,
    })
}

// Generate experiment results summary
#[allow(clippy::too_many_lines)]
async fn build_results_summary(
    experiment_id: Uuid,
    db: &impl ConnectionTrait,
) -> Result<Option<ExperimentResultsSummary>, DbErr> {
    use spice_entity::{regions, temperature_readings, well_phase_transitions, wells};

    // Get all temperature readings for this experiment to determine time span
    let temp_readings_data = temperature_readings::Entity::find()
        .filter(temperature_readings::Column::ExperimentId.eq(experiment_id))
        .order_by_asc(temperature_readings::Column::Timestamp)
        .all(db)
        .await?;

    let (first_timestamp, last_timestamp, total_time_points) = if temp_readings_data.is_empty() {
        (None, None, 0)
    } else {
        let first = temp_readings_data
            .first()
            .map(|tp| tp.timestamp.with_timezone(&Utc));
        let last = temp_readings_data
            .last()
            .map(|tp| tp.timestamp.with_timezone(&Utc));
        let count = temp_readings_data.len();
        (first, last, count)
    };

    // Create temperature readings lookup map by ID
    let temp_readings_map: std::collections::HashMap<Uuid, &temperature_readings::Model> =
        temp_readings_data.iter().map(|tr| (tr.id, tr)).collect();

    // Get all regions for this experiment
    let experiment_regions = regions::Entity::find()
        .filter(regions::Column::ExperimentId.eq(experiment_id))
        .all(db)
        .await?;

    // Get all phase transitions for this experiment
    let phase_transitions_data = well_phase_transitions::Entity::find()
        .filter(well_phase_transitions::Column::ExperimentId.eq(experiment_id))
        .find_also_related(wells::Entity)
        .all(db)
        .await?;

    // Group transitions by well to determine final states
    let mut well_final_states = std::collections::HashMap::new();
    let mut wells_with_transitions = std::collections::HashSet::new();

    for (transition, well_opt) in &phase_transitions_data {
        if let Some(well) = well_opt {
            wells_with_transitions.insert(well.id);
            // Update the final state for this well (later transitions override earlier ones)
            well_final_states.insert(well.id, transition.new_state);
        }
    }

    // Get all trays for this experiment's wells
    let tray_ids: Vec<Uuid> = phase_transitions_data
        .iter()
        .filter_map(|(_, well_opt)| well_opt.as_ref().map(|w| w.tray_id))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let trays_data = if tray_ids.is_empty() {
        vec![]
    } else {
        spice_entity::trays::Entity::find()
            .filter(spice_entity::trays::Column::Id.is_in(tray_ids))
            .all(db)
            .await?
    };

    // Create tray lookup map by ID
    let tray_map: std::collections::HashMap<Uuid, &spice_entity::trays::Model> =
        trays_data.iter().map(|t| (t.id, t)).collect();

    // Count wells by final state
    let wells_frozen = well_final_states
        .values()
        .filter(|&&state| state == 1)
        .count();
    let wells_liquid = well_final_states
        .values()
        .filter(|&&state| state == 0)
        .count();
    let wells_with_data = wells_with_transitions.len();

    // Get wells for this experiment - either from phase transitions or all wells associated with experiment
    let experiment_wells = if wells_with_transitions.is_empty() {
        // If no phase transitions, try to get wells from the experiment's tray configuration

        let experiment = experiments::Entity::find_by_id(experiment_id)
            .one(db)
            .await?;

        if let Some(exp) = experiment {
            if let Some(tray_config_id) = exp.tray_configuration_id {
                let tray_assignments = tray_configuration_assignments::Entity::find()
                    .filter(
                        tray_configuration_assignments::Column::TrayConfigurationId
                            .eq(tray_config_id),
                    )
                    .all(db)
                    .await?;

                let tray_ids: Vec<Uuid> =
                    tray_assignments.into_iter().map(|ta| ta.tray_id).collect();

                wells::Entity::find()
                    .filter(wells::Column::TrayId.is_in(tray_ids))
                    .all(db)
                    .await?
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        wells::Entity::find()
            .filter(
                wells::Column::Id.is_in(wells_with_transitions.iter().copied().collect::<Vec<_>>()),
            )
            .all(db)
            .await?
    };

    // Batch load all treatment and sample data for efficiency
    let treatment_ids: Vec<Uuid> = experiment_regions
        .iter()
        .filter_map(|r| r.treatment_id)
        .collect();

    let mut treatment_map = std::collections::HashMap::new();
    if !treatment_ids.is_empty() {
        use spice_entity::{locations, samples, treatments};

        let treatments_data = treatments::Entity::find()
            .filter(treatments::Column::Id.is_in(treatment_ids))
            .all(db)
            .await?;

        // Get all sample IDs from treatments
        let sample_ids: Vec<Uuid> = treatments_data.iter().filter_map(|t| t.sample_id).collect();

        let samples_data = if sample_ids.is_empty() {
            vec![]
        } else {
            samples::Entity::find()
                .filter(samples::Column::Id.is_in(sample_ids))
                .all(db)
                .await?
        };

        // Get all location IDs from samples
        let location_ids: Vec<Uuid> = samples_data.iter().filter_map(|s| s.location_id).collect();

        let locations_data = if location_ids.is_empty() {
            vec![]
        } else {
            locations::Entity::find()
                .filter(locations::Column::Id.is_in(location_ids))
                .all(db)
                .await?
        };

        // Build lookup maps
        let location_map: std::collections::HashMap<Uuid, &locations::Model> =
            locations_data.iter().map(|l| (l.id, l)).collect();

        let sample_map: std::collections::HashMap<
            Uuid,
            (&samples::Model, Option<&locations::Model>),
        > = samples_data
            .iter()
            .map(|s| {
                let location = s
                    .location_id
                    .and_then(|loc_id| location_map.get(&loc_id))
                    .copied();
                (s.id, (s, location))
            })
            .collect();

        // Build treatment info map
        for treatment in treatments_data {
            let sample_info = treatment.sample_id.and_then(|sample_id| {
                sample_map.get(&sample_id).map(|(sample, location)| {
                    let location_info = location.map(|l| LocationInfo {
                        id: l.id,
                        name: l.name.clone(),
                    });

                    SampleInfo {
                        id: sample.id,
                        name: sample.name.clone(),
                        location: location_info,
                    }
                })
            });

            let treatment_info = TreatmentInfo {
                id: treatment.id,
                name: treatment.name.clone(),
                notes: treatment.notes.clone(),
                enzyme_volume_litres: treatment.enzyme_volume_litres,
                sample: sample_info,
            };

            treatment_map.insert(treatment.id, treatment_info);
        }
    }

    // Build well summaries
    let mut well_summaries = Vec::new();
    for well in &experiment_wells {
        // Convert row/col to coordinate (A1, B2, etc.)s
        let coordinate = WellCoordinate {
            column: u8::try_from(well.column_number)
                .map_err(|_| DbErr::Custom("Column number out of range for u8".to_string()))?,
            row: u8::try_from(well.row_number)
                .map_err(|_| DbErr::Custom("Row number out of range for u8".to_string()))?,
        };
        let coordinate = coordinates_to_str(coordinate).map_err(|e| DbErr::Custom(e))?;
        // Get phase transitions for this well
        let well_transitions: Vec<&well_phase_transitions::Model> = phase_transitions_data
            .iter()
            .filter_map(|(transition, well_opt)| {
                if well_opt.as_ref().map(|w| w.id) == Some(well.id) {
                    Some(transition)
                } else {
                    None
                }
            })
            .collect();

        // Calculate first phase change time (first 0→1 transition) and get temperature reading
        let first_phase_change_transition = well_transitions
            .iter()
            .find(|transition| transition.previous_state == 0 && transition.new_state == 1);

        let first_phase_change_time = first_phase_change_transition
            .map(|transition| transition.timestamp.with_timezone(&Utc));

        // Get temperature probe values at first phase change
        let first_phase_change_temperature_probes = first_phase_change_transition
            .and_then(|transition| temp_readings_map.get(&transition.temperature_reading_id))
            .map(|temp_reading| {
                // Collect all non-null probe values for average calculation
                let probe_values = [
                    temp_reading.probe_1,
                    temp_reading.probe_2,
                    temp_reading.probe_3,
                    temp_reading.probe_4,
                    temp_reading.probe_5,
                    temp_reading.probe_6,
                    temp_reading.probe_7,
                    temp_reading.probe_8,
                ];

                let non_null_values: Vec<Decimal> = probe_values.into_iter().flatten().collect();

                // Calculate average if we have any values
                let average = if non_null_values.is_empty() {
                    None
                } else {
                    let sum: Decimal = non_null_values.iter().sum();
                    let avg = sum / Decimal::from(non_null_values.len());
                    // Round to 3 decimal places
                    Some(avg.round_dp(3))
                };

                TemperatureProbeValues {
                    probe_1: temp_reading.probe_1.map(|d| d.round_dp(3)),
                    probe_2: temp_reading.probe_2.map(|d| d.round_dp(3)),
                    probe_3: temp_reading.probe_3.map(|d| d.round_dp(3)),
                    probe_4: temp_reading.probe_4.map(|d| d.round_dp(3)),
                    probe_5: temp_reading.probe_5.map(|d| d.round_dp(3)),
                    probe_6: temp_reading.probe_6.map(|d| d.round_dp(3)),
                    probe_7: temp_reading.probe_7.map(|d| d.round_dp(3)),
                    probe_8: temp_reading.probe_8.map(|d| d.round_dp(3)),
                    average,
                }
            });

        // Calculate seconds from experiment start to first phase change
        let first_phase_change_seconds = match (first_phase_change_time, first_timestamp) {
            (Some(phase_change_time), Some(start_time)) => {
                Some((phase_change_time - start_time).num_seconds())
            }
            _ => None,
        };

        // Determine final state
        let final_state = well_final_states
            .get(&well.id)
            .map(|&state| match state {
                0 => "liquid".to_string(),
                1 => "frozen".to_string(),
                _ => "unknown".to_string(),
            })
            .or_else(|| Some("no_data".to_string()));

        // Find region for this well to get sample/treatment info
        // Convert 1-based well coordinates to 0-based for region comparison
        let well_row_0based = well.row_number - 1;
        let well_col_0based = well.column_number - 1;

        let region = experiment_regions.iter().find(|r| {
            if let (Some(row_min), Some(row_max), Some(col_min), Some(col_max)) =
                (r.row_min, r.row_max, r.col_min, r.col_max)
            {
                well_row_0based >= row_min
                    && well_row_0based <= row_max
                    && well_col_0based >= col_min
                    && well_col_0based <= col_max
            } else {
                false
            }
        });
        // Get treatment info if region exists
        let treatment_info = region
            .and_then(|r| r.treatment_id)
            .and_then(|treatment_id| treatment_map.get(&treatment_id).cloned());

        // Get tray information
        let tray_info = tray_map.get(&well.tray_id);
        let tray_name = tray_info.and_then(|t| t.name.clone());

        let well_summary = WellSummary {
            row: well.row_number,
            col: well.column_number,
            coordinate,
            first_phase_change_time,
            first_phase_change_seconds,
            first_phase_change_temperature_probes,
            final_state,
            tray_id: Some(well.tray_id.to_string()),
            tray_name,
            dilution_factor: region.and_then(|r| r.dilution_factor),
            treatment: treatment_info,
        };

        well_summaries.push(well_summary);
    }

    // Always return a summary, even if empty
    Ok(Some(ExperimentResultsSummary {
        total_wells: experiment_wells.len(),
        wells_with_data,
        wells_frozen,
        wells_liquid,
        total_time_points,
        first_timestamp,
        last_timestamp,
        well_summaries,
    }))
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct TreatmentInfo {
    pub id: Uuid,
    pub name: TreatmentName,
    pub notes: Option<String>,
    pub enzyme_volume_litres: Option<Decimal>,
    pub sample: Option<SampleInfo>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct SampleInfo {
    pub id: Uuid,
    pub name: String,
    pub location: Option<LocationInfo>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct LocationInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct TemperatureProbeValues {
    pub probe_1: Option<Decimal>,
    pub probe_2: Option<Decimal>,
    pub probe_3: Option<Decimal>,
    pub probe_4: Option<Decimal>,
    pub probe_5: Option<Decimal>,
    pub probe_6: Option<Decimal>,
    pub probe_7: Option<Decimal>,
    pub probe_8: Option<Decimal>,
    pub average: Option<Decimal>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct WellSummary {
    pub row: i32,
    pub col: i32,
    pub coordinate: String, // e.g., "A1", "B2"
    pub first_phase_change_time: Option<DateTime<Utc>>,
    pub first_phase_change_seconds: Option<i64>, // seconds from experiment start
    pub first_phase_change_temperature_probes: Option<TemperatureProbeValues>, // Temperature probe values at first phase change
    pub final_state: Option<String>, // "frozen", "liquid", "no_data"
    // pub sample_name: Option<String>,
    // pub treatment_name: Option<String>,
    // pub treatment_id: Option<Uuid>,
    pub tray_id: Option<String>, // UUID of the tray
    pub tray_name: Option<String>,
    pub dilution_factor: Option<i32>,
    // Full objects with UUIDs for UI linking
    pub treatment: Option<TreatmentInfo>,
    // pub sample: Option<SampleInfo>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct ExperimentResultsSummary {
    pub total_wells: usize,
    pub wells_with_data: usize,
    pub wells_frozen: usize,
    pub wells_liquid: usize,
    pub total_time_points: usize,
    pub first_timestamp: Option<DateTime<Utc>>,
    pub last_timestamp: Option<DateTime<Utc>>,
    pub well_summaries: Vec<WellSummary>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct TrayInfo {
    pub id: Uuid,
    pub name: Option<String>,
    pub sequence_id: i32,
    pub qty_x_axis: Option<i32>,
    pub qty_y_axis: Option<i32>,
    pub well_relative_diameter: Option<String>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct RegionInput {
    pub name: Option<String>,
    pub tray_sequence_id: Option<i32>, // Renamed from tray_id for clarity
    pub col_min: Option<i32>,
    pub col_max: Option<i32>,
    pub row_min: Option<i32>,
    pub row_max: Option<i32>,
    pub color: Option<String>, // hex color
    pub dilution: Option<String>,
    pub treatment_id: Option<Uuid>,
    pub is_background_key: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub treatment: Option<TreatmentInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tray: Option<TrayInfo>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
struct TrayRegions {
    pub id: Uuid,
    pub experiment_id: Uuid,
    pub treatment_id: Option<Uuid>,
    pub name: Option<String>,
    pub display_colour_hex: Option<String>,
    pub tray_id: Option<i32>,
    pub col_min: Option<i32>,
    pub row_min: Option<i32>,
    pub col_max: Option<i32>,
    pub row_max: Option<i32>,
    pub dilution_factor: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl From<spice_entity::regions::Model> for TrayRegions {
    fn from(region: spice_entity::regions::Model) -> Self {
        Self {
            id: region.id,
            experiment_id: region.experiment_id,
            treatment_id: region.treatment_id,
            name: region.name,
            display_colour_hex: region.display_colour_hex,
            tray_id: region.tray_id,
            col_min: region.col_min,
            row_min: region.row_min,
            col_max: region.col_max,
            row_max: region.row_max,
            dilution_factor: region.dilution_factor,
            created_at: region.created_at.into(),
            last_updated: region.last_updated.into(),
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "spice_entity::experiments::ActiveModel"]
pub struct Experiment {
    #[crudcrate(update_model = false, create_model = false, on_create = Uuid::new_v4())]
    id: Uuid,
    name: String,
    // sample_id: Uuid,
    tray_configuration_id: Option<Uuid>,
    username: Option<String>,
    performed_at: Option<DateTime<Utc>>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    last_updated: DateTime<Utc>,
    temperature_ramp: Option<Decimal>,
    temperature_start: Option<Decimal>,
    temperature_end: Option<Decimal>,
    is_calibration: bool,
    remarks: Option<String>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    assets: Vec<crate::routes::assets::models::Asset>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    regions: Vec<RegionInput>,
    #[crudcrate(non_db_attr = true, default = None)]
    results_summary: Option<ExperimentResultsSummary>,
}

impl From<Model> for Experiment {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            tray_configuration_id: model.tray_configuration_id,
            username: model.username,
            performed_at: model.performed_at.map(|dt| dt.with_timezone(&Utc)),
            created_at: model.created_at.into(),
            last_updated: model.last_updated.into(),
            temperature_ramp: model.temperature_ramp,
            temperature_start: model.temperature_start,
            temperature_end: model.temperature_end,
            is_calibration: model.is_calibration,
            remarks: model.remarks,
            assets: vec![],
            regions: vec![],
            results_summary: None,
        }
    }
}

#[async_trait]
impl CRUDResource for Experiment {
    type EntityType = spice_entity::experiments::Entity;
    type ColumnType = spice_entity::experiments::Column;
    type ActiveModelType = spice_entity::experiments::ActiveModel;
    type CreateModel = ExperimentCreate;
    type UpdateModel = ExperimentUpdate;

    const ID_COLUMN: Self::ColumnType = spice_entity::experiments::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "experiments";
    const RESOURCE_NAME_SINGULAR: &'static str = "experiment";
    const RESOURCE_DESCRIPTION: &'static str =
        "This resource manages experiments associated with sample data collected during campaigns.";

    async fn get_one(db: &DatabaseConnection, id: Uuid) -> Result<Self, DbErr> {
        let model =
            Self::EntityType::find_by_id(id)
                .one(db)
                .await?
                .ok_or(DbErr::RecordNotFound(format!(
                    "{} not found",
                    Self::RESOURCE_NAME_SINGULAR
                )))?;

        let s3_assets = model
            .find_related(spice_entity::s3_assets::Entity)
            .all(db)
            .await?;

        let regions = model
            .find_related(spice_entity::regions::Entity)
            .all(db)
            .await?;

        let mut regions_with_treatment = Vec::new();
        for region in regions {
            let region_input = region_model_to_input_with_treatment(region, db).await?;
            regions_with_treatment.push(region_input);
        }

        // Build results summary
        let results_summary = build_results_summary(id, db).await?;

        let mut model: Self = model.into();
        model.assets = s3_assets.into_iter().map(Into::into).collect();
        model.regions = regions_with_treatment;
        model.results_summary = results_summary;

        Ok(model)
    }

    async fn create(db: &DatabaseConnection, data: Self::CreateModel) -> Result<Self, DbErr> {
        let txn = db.begin().await?;

        // Store regions before conversion since they're not part of the DB model
        let regions_to_create = data.regions.clone();

        // Create the experiment first
        let experiment_model: Self::ActiveModelType = data.into();
        let experiment = experiment_model.insert(&txn).await?;

        // Handle regions if provided
        if !regions_to_create.is_empty() {
            let region_models = create_region_active_models(experiment.id, regions_to_create, &txn);

            for region_model in region_models {
                region_model.insert(&txn).await?;
            }
        }

        txn.commit().await?;

        // Return the complete experiment with regions
        Self::get_one(db, experiment.id).await
    }

    async fn update(
        db: &DatabaseConnection,
        id: Uuid,
        update_data: Self::UpdateModel,
    ) -> Result<Self, DbErr> {
        let txn = db.begin().await?;

        let existing: Self::ActiveModelType = Self::EntityType::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_PLURAL
            )))?
            .into();

        let updated_model = update_data.clone().merge_into_activemodel(existing);
        let _updated = updated_model.update(&txn).await?;

        // Handle regions update - delete existing regions and create new ones
        if !update_data.regions.is_empty() {
            // Delete existing regions for this experiment
            spice_entity::regions::Entity::delete_many()
                .filter(spice_entity::regions::Column::ExperimentId.eq(id))
                .exec(&txn)
                .await?;

            // Create new regions
            let region_models = create_region_active_models(id, update_data.regions.clone(), &txn);

            for region_model in region_models {
                region_model.insert(&txn).await?;
            }
        }

        txn.commit().await?;

        // Return the complete experiment with regions
        Self::get_one(db, id).await
    }

    async fn get_all(
        db: &DatabaseConnection,
        condition: Condition,
        order_column: Self::ColumnType,
        order_direction: Order,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Self>, DbErr> {
        let models = Self::EntityType::find()
            .filter(condition)
            .order_by(order_column, order_direction)
            .offset(offset)
            .limit(limit)
            .all(db)
            .await?;

        let mut experiments = Vec::new();

        for model in models {
            let s3_assets = model
                .find_related(spice_entity::s3_assets::Entity)
                .all(db)
                .await?;

            let regions = model
                .find_related(spice_entity::regions::Entity)
                .all(db)
                .await?;

            let mut regions_with_treatment = Vec::new();
            for region in regions {
                let region_input = region_model_to_input_with_treatment(region, db).await?;
                regions_with_treatment.push(region_input);
            }

            let mut experiment: Self = model.into();
            experiment.assets = s3_assets.into_iter().map(Into::into).collect();
            experiment.regions = regions_with_treatment;

            experiments.push(experiment);
        }

        Ok(experiments)
    }

    fn sortable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("id", Self::ColumnType::Id),
            ("name", Self::ColumnType::Name),
            ("performed_at", Self::ColumnType::PerformedAt),
            ("username", Self::ColumnType::Username),
            ("created_at", Self::ColumnType::CreatedAt),
            ("temperature_ramp", Self::ColumnType::TemperatureRamp),
            ("temperature_start", Self::ColumnType::TemperatureStart),
            ("temperature_end", Self::ColumnType::TemperatureEnd),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("name", Self::ColumnType::Name),
            ("performed_at", Self::ColumnType::PerformedAt),
            ("username", Self::ColumnType::Username),
            ("created_at", Self::ColumnType::CreatedAt),
            ("temperature_ramp", Self::ColumnType::TemperatureRamp),
            ("temperature_start", Self::ColumnType::TemperatureStart),
            ("temperature_end", Self::ColumnType::TemperatureEnd),
        ]
    }
}
