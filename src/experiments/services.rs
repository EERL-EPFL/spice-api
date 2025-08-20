use super::models::{
    ExperimentResultsResponse, ExperimentResultsSummaryCompact, TrayResultsSummary, TrayWellSummary,
};
use crate::{
    experiments::models as experiments,
    experiments::phase_transitions::models as well_phase_transitions,
    experiments::temperatures::models as temperature_readings,
    tray_configurations::regions::models as regions, tray_configurations::trays::models as trays,
    tray_configurations::wells::models as wells,
};
use crate::{
    locations::models as locations, samples::models as samples, treatments::models as treatments,
};
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
    temp_readings_map: &'a std::collections::HashMap<Uuid, temperature_readings::Model>,
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

// Helper to calculate average from temperature probes and create formatted reading
fn format_temperature_reading(
    temp_reading: &temperature_readings::Model,
) -> temperature_readings::Model {
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
    let average = if non_null_values.is_empty() {
        None
    } else {
        Some(
            (non_null_values.iter().sum::<Decimal>() / Decimal::from(non_null_values.len()))
                .round_dp(3),
        )
    };

    let mut formatted = temp_reading.clone();
    formatted.probe_1 = temp_reading.probe_1.map(|d| d.round_dp(3));
    formatted.probe_2 = temp_reading.probe_2.map(|d| d.round_dp(3));
    formatted.probe_3 = temp_reading.probe_3.map(|d| d.round_dp(3));
    formatted.probe_4 = temp_reading.probe_4.map(|d| d.round_dp(3));
    formatted.probe_5 = temp_reading.probe_5.map(|d| d.round_dp(3));
    formatted.probe_6 = temp_reading.probe_6.map(|d| d.round_dp(3));
    formatted.probe_7 = temp_reading.probe_7.map(|d| d.round_dp(3));
    formatted.probe_8 = temp_reading.probe_8.map(|d| d.round_dp(3));
    formatted.average = average;
    formatted
}

// Helper function to load temperature readings and calculate time span
async fn load_temperature_data(
    experiment_id: Uuid,
    db: &impl ConnectionTrait,
) -> Result<
    (
        std::collections::HashMap<Uuid, temperature_readings::Model>,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
        usize,
    ),
    DbErr,
> {
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
    let temp_readings_map: std::collections::HashMap<Uuid, temperature_readings::Model> =
        temp_readings_data
            .iter()
            .map(|tr| (tr.id, tr.clone()))
            .collect();

    Ok((
        temp_readings_map,
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
    wells_with_transitions: &std::collections::HashSet<Uuid>,
    phase_transitions_data: &[(well_phase_transitions::Model, Option<wells::Model>)],
    db: &impl ConnectionTrait,
) -> Result<
    (
        Vec<wells::Model>,
        std::collections::HashMap<Uuid, trays::Model>,
    ),
    DbErr,
> {
    // Get wells for this experiment
    let experiment_wells = if wells_with_transitions.is_empty() {
        let experiment = experiments::Entity::find_by_id(experiment_id)
            .one(db)
            .await?;

        if let Some(exp) = experiment {
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
        }
    } else {
        wells::Entity::find()
            .filter(
                wells::Column::Id.is_in(wells_with_transitions.iter().copied().collect::<Vec<_>>()),
            )
            .all(db)
            .await?
    };

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
        trays::Entity::find()
            .filter(trays::Column::Id.is_in(tray_ids))
            .all(db)
            .await?
    };

    let tray_map: std::collections::HashMap<Uuid, trays::Model> =
        trays_data.iter().map(|t| (t.id, t.clone())).collect();

    Ok((experiment_wells, tray_map))
}

// Helper function to load treatment and sample data
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

    let mut treatment_map = std::collections::HashMap::new();

    if treatment_ids.is_empty() {
        return Ok(treatment_map);
    }

    let treatments_data = treatments::Entity::find()
        .filter(treatments::Column::Id.is_in(treatment_ids))
        .all(db)
        .await?;

    let sample_ids: Vec<Uuid> = treatments_data.iter().filter_map(|t| t.sample_id).collect();

    let samples_data = if sample_ids.is_empty() {
        vec![]
    } else {
        samples::Entity::find()
            .filter(samples::Column::Id.is_in(sample_ids))
            .all(db)
            .await?
    };

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

    let sample_map: std::collections::HashMap<Uuid, (&samples::Model, Option<&locations::Model>)> =
        samples_data
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
    for treatment in &treatments_data {
        let sample_info = treatment.sample_id.and_then(|sample_id| {
            sample_map.get(&sample_id).map(|(sample, _location)| {
                let sample_api: crate::samples::models::Sample = (*sample).clone().into();
                sample_api
            })
        });

        let treatment_info: crate::treatments::models::Treatment = treatment.clone().into();

        treatment_map.insert(treatment.id, (treatment_info, sample_info));
    }

    Ok(treatment_map)
}

pub(super) async fn build_tray_centric_results(
    experiment_id: Uuid,
    db: &impl ConnectionTrait,
) -> Result<Option<ExperimentResultsResponse>, DbErr> {
    // Load all required data using existing helper functions
    let (temp_readings_map, first_timestamp, last_timestamp, total_time_points) =
        load_temperature_data(experiment_id, db).await?;

    let filename_to_asset_id = load_experiment_assets(experiment_id, db).await?;

    let experiment_regions = regions::Entity::find()
        .filter(regions::Column::ExperimentId.eq(experiment_id))
        .all(db)
        .await?;

    let (phase_transitions_data, wells_with_transitions) =
        process_phase_transitions(experiment_id, db).await?;

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

fn build_tray_summaries(context: &WellSummaryContext) -> Vec<TrayResultsSummary> {
    // Group wells by tray
    let mut tray_wells: std::collections::HashMap<Uuid, Vec<&wells::Model>> =
        std::collections::HashMap::new();

    for well in context.experiment_wells {
        tray_wells.entry(well.tray_id).or_default().push(well);
    }

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

            // Count total phase changes
            let total_phase_changes = well_transitions.len();

            // Get temperature probe data at first phase change
            let temperatures: Option<temperature_readings::TemperatureReading> =
                first_phase_change_transition
                    .and_then(|transition| {
                        context
                            .temp_readings_map
                            .get(&transition.temperature_reading_id)
                    })
                    .map(|temp_reading| format_temperature_reading(temp_reading).into());
            let image_filename: Option<String> =
                temperatures.clone().and_then(|t| t.image_filename);
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

            let treatment_name = treatment.as_ref().map(|t| match t.name {
                crate::treatments::models::TreatmentName::None => "none".to_string(),
                crate::treatments::models::TreatmentName::Heat => "heat".to_string(),
                crate::treatments::models::TreatmentName::H2o2 => "h2o2".to_string(),
            });

            let tray_well_summary = TrayWellSummary {
                row_letter: well.row_letter.clone(),
                column_number: well.column_number,
                coordinate,
                sample,
                treatment_name,
                treatment,
                dilution_factor: region.and_then(|r| r.dilution_factor),
                first_phase_change_time,
                temperatures,
                total_phase_changes,
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
