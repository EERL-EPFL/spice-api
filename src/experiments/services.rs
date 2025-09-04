use super::models::{
    ExperimentResultsResponse, ExperimentResultsSummaryCompact, TemperatureDataWithProbes,
    TrayResultsSummary, TrayWellSummary,
};
use crate::{
    experiments::models as experiments,
    experiments::phase_transitions::models as well_phase_transitions,
    experiments::probe_temperature_readings::models as probe_temperature_readings,
    experiments::temperatures::models as temperature_readings,
    tray_configurations::probes::models as probes, tray_configurations::regions::models as regions,
    tray_configurations::trays::models as trays, tray_configurations::wells::models as wells,
};
use crate::{samples::models as samples, treatments::models as treatments};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, EntityTrait, QueryOrder, entity::prelude::*};
use uuid::Uuid;

// Constants for phase states
const PHASE_LIQUID: i32 = 0;
const PHASE_FROZEN: i32 = 1;

// Parameter struct to reduce argument count in build_well_summaries
struct WellSummaryContext<'a> {
    experiment_wells: &'a [wells::Model],
    phase_transitions_data: &'a [(well_phase_transitions::Model, Option<wells::Model>)],
    temp_readings_map: &'a std::collections::HashMap<Uuid, TemperatureDataWithProbes>,
    filename_to_asset_id: &'a std::collections::HashMap<String, Uuid>,
    experiment_regions: &'a [regions::Model],
    treatment_map: &'a std::collections::HashMap<
        Uuid,
        (
            crate::treatments::models::Treatment,
            Option<crate::samples::models::Sample>,
        ),
    >,
    tray_map: &'a std::collections::HashMap<Uuid, trays::Model>,
}

// Helper function to convert row letter to 0-based index
fn row_letter_to_index(row_letter: &str) -> i32 {
    row_letter
        .chars()
        .next()
        .map_or(0, |c| c as i32 - 'A' as i32)
}

// Helper function to load temperature readings with individual probe data (optimized to only load needed readings)
#[allow(clippy::too_many_lines)] // Complex data loading logic requires extensive processing
async fn load_individual_temperature_data(
    experiment_id: Uuid,
    phase_transition_temp_ids: &std::collections::HashSet<Uuid>,
    db: &impl ConnectionTrait,
) -> Result<
    (
        std::collections::HashMap<Uuid, TemperatureDataWithProbes>,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
        usize,
    ),
    DbErr,
> {
    // Only load temperature readings that we actually need (for phase transitions)
    let temp_reading_ids_vec: Vec<Uuid> = phase_transition_temp_ids.iter().copied().collect();

    // Get total count of temperature readings for summary stats
    let total_time_points = {
        let count = temperature_readings::Entity::find()
            .filter(temperature_readings::Column::ExperimentId.eq(experiment_id))
            .count(db)
            .await?;
        usize::try_from(count)
            .map_err(|_| DbErr::Custom("Temperature readings count exceeds maximum".to_string()))?
    };

    // Get first and last timestamps for summary (lightweight query)
    let first_temp_reading = temperature_readings::Entity::find()
        .filter(temperature_readings::Column::ExperimentId.eq(experiment_id))
        .order_by_asc(temperature_readings::Column::Timestamp)
        .one(db)
        .await?;

    let last_temp_reading = temperature_readings::Entity::find()
        .filter(temperature_readings::Column::ExperimentId.eq(experiment_id))
        .order_by_desc(temperature_readings::Column::Timestamp)
        .one(db)
        .await?;

    let first_timestamp = first_temp_reading.map(|tr| tr.timestamp.with_timezone(&Utc));
    let last_timestamp = last_temp_reading.map(|tr| tr.timestamp.with_timezone(&Utc));

    // Only load the specific temperature readings we need (192 instead of 6,786)
    let temp_readings_data = if temp_reading_ids_vec.is_empty() {
        vec![]
    } else {
        temperature_readings::Entity::find()
            .filter(temperature_readings::Column::Id.is_in(temp_reading_ids_vec))
            .order_by_asc(temperature_readings::Column::Timestamp)
            .all(db)
            .await?
    };

    if temp_readings_data.is_empty() {
        return Ok((
            std::collections::HashMap::new(),
            first_timestamp,
            last_timestamp,
            total_time_points,
        ));
    }

    // Get the experiment to find its tray configuration
    let experiment = experiments::Entity::find_by_id(experiment_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Experiment not found".to_string()))?;

    // Load all probes from the tray configuration for this experiment
    let all_experiment_probes = if let Some(tray_config_id) = experiment.tray_configuration_id {
        // Get all trays for this configuration
        let trays = trays::Entity::find()
            .filter(trays::Column::TrayConfigurationId.eq(tray_config_id))
            .all(db)
            .await?;

        let tray_ids: Vec<Uuid> = trays.iter().map(|t| t.id).collect();

        // Get all probes from all trays in this configuration
        probes::Entity::find()
            .filter(probes::Column::TrayId.is_in(tray_ids))
            .all(db)
            .await?
    } else {
        Vec::new()
    };

    // Get individual probe readings for all temperature readings
    let temp_reading_ids: Vec<Uuid> = temp_readings_data.iter().map(|tr| tr.id).collect();

    let probe_readings = probe_temperature_readings::Entity::find()
        .filter(probe_temperature_readings::Column::TemperatureReadingId.is_in(temp_reading_ids))
        .find_also_related(probes::Entity)
        .all(db)
        .await?;

    // Group probe readings by temperature_reading_id
    let mut probe_readings_by_temp_id: std::collections::HashMap<
        Uuid,
        Vec<(probe_temperature_readings::Model, Option<probes::Model>)>,
    > = std::collections::HashMap::new();

    for (probe_reading, probe_opt) in probe_readings {
        probe_readings_by_temp_id
            .entry(probe_reading.temperature_reading_id)
            .or_default()
            .push((probe_reading, probe_opt));
    }

    // Build temperature data with probes using crudcrate models
    let mut temp_data_map = std::collections::HashMap::new();

    for temp_reading in &temp_readings_data {
        let empty_vec = Vec::new();
        let actual_probe_readings = probe_readings_by_temp_id
            .get(&temp_reading.id)
            .unwrap_or(&empty_vec);

        // Create a HashMap of actual readings by probe_id for quick lookup
        let mut readings_by_probe_id: std::collections::HashMap<Uuid, Decimal> =
            std::collections::HashMap::new();
        for (probe_reading, probe_opt) in actual_probe_readings {
            if let Some(probe) = probe_opt {
                readings_by_probe_id.insert(probe.id, probe_reading.temperature);
            }
        }

        // Create complete probe readings array including ALL probes from tray configuration
        let mut complete_probe_readings = Vec::new();
        let mut temperature_values = Vec::new();

        for probe in &all_experiment_probes {
            let temperature_value = readings_by_probe_id.get(&probe.id).copied();

            // Only include probe readings that have actual temperature data
            // This avoids showing misleading "0" temperatures for probes without readings
            if let Some(actual_temp) = temperature_value {
                // Create probe temperature reading with metadata (rounded to 3 decimal places)
                let probe_temp_reading = super::models::ProbeTemperatureReadingWithMetadata {
                    id: uuid::Uuid::new_v4(), // Placeholder ID for API response
                    temperature_reading_id: temp_reading.id,
                    temperature: actual_temp.round_dp(3), // Round to 3 decimal places
                    created_at: temp_reading.created_at,
                    // Probe metadata
                    probe_id: probe.id,
                    probe_name: probe.name.clone(),
                    probe_data_column_index: probe.data_column_index,
                    probe_position_x: probe.position_x,
                    probe_position_y: probe.position_y,
                };

                complete_probe_readings.push(probe_temp_reading);
                temperature_values.push(actual_temp);
            }
        }

        // Calculate average temperature from actual probe readings only (rounded to 3 decimal places)
        let temperature_average = if temperature_values.is_empty() {
            None
        } else {
            let sum: Decimal = temperature_values.iter().sum();
            let average = sum / Decimal::from(temperature_values.len());
            // Round to 3 decimal places
            Some(average.round_dp(3))
        };

        // Create flattened temperature data with ALL probe readings from tray configuration
        let temp_data_with_probes = TemperatureDataWithProbes {
            id: temp_reading.id,
            experiment_id: temp_reading.experiment_id,
            timestamp: temp_reading.timestamp,
            image_filename: temp_reading.image_filename.clone(),
            average: temperature_average,
            probe_readings: complete_probe_readings,
        };

        temp_data_map.insert(temp_reading.id, temp_data_with_probes);
    }

    // Summary timestamps and count already calculated above

    Ok((
        temp_data_map,
        first_timestamp,
        last_timestamp,
        total_time_points,
    ))
}

// Helper function to load experiment assets and create filename mapping
async fn load_experiment_assets(
    experiment_id: Uuid,
    db: &impl ConnectionTrait,
) -> Result<std::collections::HashMap<String, Uuid>, DbErr> {
    let experiment_assets = crate::assets::models::Entity::find()
        .filter(crate::assets::models::Column::ExperimentId.eq(experiment_id))
        .filter(crate::assets::models::Column::Type.eq("image"))
        .all(db)
        .await?;

    // Create filename-to-asset-id mapping (strip .jpg extension for matching)
    let filename_to_asset_id: std::collections::HashMap<String, Uuid> = experiment_assets
        .iter()
        .map(|asset| {
            let filename_without_ext = if std::path::Path::new(&asset.original_filename)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("jpg"))
            {
                asset
                    .original_filename
                    .strip_suffix(".jpg")
                    .unwrap_or(&asset.original_filename)
                    .to_string()
            } else {
                asset.original_filename.clone()
            };
            (filename_without_ext, asset.id)
        })
        .collect();

    Ok(filename_to_asset_id)
}

// Helper function to process phase transitions and determine well states
async fn process_phase_transitions(
    experiment_id: Uuid,
    db: &impl ConnectionTrait,
) -> Result<
    (
        Vec<(well_phase_transitions::Model, Option<wells::Model>)>,
        std::collections::HashSet<Uuid>,
    ),
    DbErr,
> {
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

    Ok((phase_transitions_data, wells_with_transitions))
}

// Helper function to load experiment wells and trays
async fn load_experiment_wells_and_trays(
    experiment_id: Uuid,
    _wells_with_transitions: &std::collections::HashSet<Uuid>,
    _phase_transitions_data: &[(well_phase_transitions::Model, Option<wells::Model>)],
    db: &impl ConnectionTrait,
) -> Result<
    (
        Vec<wells::Model>,
        std::collections::HashMap<Uuid, trays::Model>,
    ),
    DbErr,
> {
    // Always load ALL wells for this experiment from tray configuration
    // This ensures we show both wells that froze and wells that never froze (important scientific data)
    let experiment = experiments::Entity::find_by_id(experiment_id)
        .one(db)
        .await?;

    let experiment_wells = if let Some(exp) = experiment {
        if let Some(tray_config_id) = exp.tray_configuration_id {
            let tray_list = trays::Entity::find()
                .filter(trays::Column::TrayConfigurationId.eq(tray_config_id))
                .all(db)
                .await?;

            let tray_ids: Vec<Uuid> = tray_list.into_iter().map(|tray| tray.id).collect();

            wells::Entity::find()
                .filter(wells::Column::TrayId.is_in(tray_ids))
                .all(db)
                .await?
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Get all trays for this experiment's wells (use actual experiment wells, not just transition wells)
    let tray_ids: Vec<Uuid> = experiment_wells
        .iter()
        .map(|w| w.tray_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let trays_data = if tray_ids.is_empty() {
        vec![]
    } else {
        trays::Entity::find()
            .filter(trays::Column::Id.is_in(tray_ids))
            .all(db)
            .await?
    };

    let tray_map: std::collections::HashMap<Uuid, trays::Model> =
        trays_data.iter().map(|t| (t.id, t.clone())).collect();

    Ok((experiment_wells, tray_map))
}

// Helper function to load treatment and sample data with optimized joins
async fn load_treatment_and_sample_data(
    experiment_regions: &[regions::Model],
    db: &impl ConnectionTrait,
) -> Result<
    std::collections::HashMap<
        Uuid,
        (
            crate::treatments::models::Treatment,
            Option<crate::samples::models::Sample>,
        ),
    >,
    DbErr,
> {
    let treatment_ids: Vec<Uuid> = experiment_regions
        .iter()
        .filter_map(|r| r.treatment_id)
        .collect();

    if treatment_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    // Load treatments with their related samples in a single query using eager loading
    let treatments_with_samples = treatments::Entity::find()
        .filter(treatments::Column::Id.is_in(treatment_ids))
        .find_with_related(samples::Entity)
        .all(db)
        .await?;

    let mut treatment_map = std::collections::HashMap::new();

    for (treatment_model, sample_models) in treatments_with_samples {
        let treatment_info: crate::treatments::models::Treatment = treatment_model.into();

        // Get the first (and only) sample for this treatment
        let sample_info = sample_models.into_iter().next().map(|sample| {
            let sample_api: crate::samples::models::Sample = sample.into();
            sample_api
        });

        treatment_map.insert(treatment_info.id, (treatment_info, sample_info));
    }

    Ok(treatment_map)
}

pub async fn build_tray_centric_results(
    experiment_id: Uuid,
    db: &impl ConnectionTrait,
) -> Result<Option<ExperimentResultsResponse>, DbErr> {
    // First load phase transitions to get the temperature reading IDs we actually need
    let (phase_transitions_data, wells_with_transitions) =
        process_phase_transitions(experiment_id, db).await?;

    // Extract temperature reading IDs from phase transitions (only ~192 instead of 6,786)
    let phase_transition_temp_ids: std::collections::HashSet<Uuid> = phase_transitions_data
        .iter()
        .map(|(transition, _)| transition.temperature_reading_id)
        .collect();

    // Load temperature data only for the readings we actually need
    let (temp_readings_map, first_timestamp, last_timestamp, total_time_points) =
        load_individual_temperature_data(experiment_id, &phase_transition_temp_ids, db).await?;

    let filename_to_asset_id = load_experiment_assets(experiment_id, db).await?;

    let experiment_regions = regions::Entity::find()
        .filter(regions::Column::ExperimentId.eq(experiment_id))
        .all(db)
        .await?;

    // Phase transitions already loaded above for optimization

    let (experiment_wells, tray_map) = load_experiment_wells_and_trays(
        experiment_id,
        &wells_with_transitions,
        &phase_transitions_data,
        db,
    )
    .await?;

    let treatment_map = load_treatment_and_sample_data(&experiment_regions, db).await?;

    // Create context for shared data
    let context = WellSummaryContext {
        experiment_wells: &experiment_wells,
        phase_transitions_data: &phase_transitions_data,
        temp_readings_map: &temp_readings_map,
        filename_to_asset_id: &filename_to_asset_id,
        experiment_regions: &experiment_regions,
        treatment_map: &treatment_map,
        tray_map: &tray_map,
    };

    // Build tray-centric results using same context as well summaries
    let tray_results = build_tray_summaries(&context);

    // Create compact summary
    let summary = ExperimentResultsSummaryCompact {
        total_time_points,
        first_timestamp,
        last_timestamp,
    };

    Ok(Some(ExperimentResultsResponse {
        summary,
        trays: tray_results,
    }))
}

fn create_tray_well_hashmap(
    context: &WellSummaryContext,
) -> std::collections::HashMap<Uuid, Vec<wells::Model>> {
    let mut tray_well_map: std::collections::HashMap<Uuid, Vec<wells::Model>> =
        std::collections::HashMap::new();

    for well in context.experiment_wells {
        tray_well_map
            .entry(well.tray_id)
            .or_default()
            .push(well.clone());
    }

    tray_well_map
}

fn build_tray_summaries(context: &WellSummaryContext) -> Vec<TrayResultsSummary> {
    // Group wells by tray
    let tray_wells = create_tray_well_hashmap(context);
    let mut tray_results = Vec::new();

    for (tray_id, wells_in_tray) in tray_wells {
        let tray_info = context.tray_map.get(&tray_id);
        let tray_name = tray_info.and_then(|t| t.name.clone());

        let mut tray_well_summaries = Vec::new();
        for well in wells_in_tray {
            // Build well summary similar to existing logic but simplified
            let coordinate = format!("{}{}", well.row_letter, well.column_number);

            // Get phase transitions for this well
            let well_transitions: Vec<&well_phase_transitions::Model> = context
                .phase_transitions_data
                .iter()
                .filter_map(|(transition, well_opt)| {
                    if well_opt.as_ref().map(|w| w.id) == Some(well.id) {
                        Some(transition)
                    } else {
                        None
                    }
                })
                .collect();

            // Calculate first phase change time (first 0→1 transition)
            let first_phase_change_transition = well_transitions.iter().find(|transition| {
                transition.previous_state == PHASE_LIQUID && transition.new_state == PHASE_FROZEN
            });
            let first_phase_change_time = first_phase_change_transition
                .map(|transition| transition.timestamp.with_timezone(&Utc));

            // Get temperature data with individual probes at first phase change
            let temperatures: Option<TemperatureDataWithProbes> = first_phase_change_transition
                .and_then(|transition| {
                    context
                        .temp_readings_map
                        .get(&transition.temperature_reading_id)
                })
                .cloned();
            let image_filename: Option<String> =
                temperatures.as_ref().and_then(|t| t.image_filename.clone());
            let image_asset_id = match image_filename {
                Some(filename) => context.filename_to_asset_id.get(&filename).copied(),
                None => None,
            };
            // Simple state mapping

            // Find region for this well to get sample/treatment info
            let well_row_0based = row_letter_to_index(&well.row_letter);
            let well_col_0based = well.column_number - 1;

            let region = context.experiment_regions.iter().find(|r| {
                // First check if the well's tray matches the region's tray
                let tray_matches = if let Some(region_tray_id) = r.tray_id {
                    // Find the tray info for this well to get its sequence number
                    if let Some(tray_info) = context.tray_map.get(&well.tray_id) {
                        // Compare region tray_id (1-based sequence) with tray sequence from database
                        tray_info.order_sequence == region_tray_id
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Only check coordinates if tray matches
                if tray_matches {
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
                } else {
                    false
                }
            });

            // Get treatment and sample info if region exists
            let (treatment, sample) = region
                .and_then(|r| r.treatment_id)
                .and_then(|treatment_id| context.treatment_map.get(&treatment_id))
                .map_or((None, None), |(t, s)| (Some(t.clone()), s.clone()));

            let tray_well_summary = TrayWellSummary {
                row_letter: well.row_letter.clone(),
                column_number: well.column_number,
                coordinate,
                sample,
                treatment,
                dilution_factor: region.and_then(|r| r.dilution_factor),
                first_phase_change_time,
                temperatures,
                total_phase_changes: well_transitions.len(),
                image_asset_id,
            };

            tray_well_summaries.push(tray_well_summary);
        }

        let tray_summary = TrayResultsSummary {
            tray_id: tray_id.to_string(),
            tray_name,
            wells: tray_well_summaries,
        };
        tray_results.push(tray_summary);
    }

    // Sort trays by their sequence or name
    tray_results.sort_by(|a, b| a.tray_name.cmp(&b.tray_name));
    tray_results
}
