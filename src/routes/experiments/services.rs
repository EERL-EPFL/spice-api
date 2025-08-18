use super::models::{ExperimentResultsSummary, SampleResultsSummary, TreatmentResultsSummary, WellSummary};
// Coordinate transformation functions no longer needed - wells store alphanumeric coordinates directly
use crate::routes::{
    experiments::models as experiments,
    experiments::phase_transitions::models as well_phase_transitions,
    experiments::temperatures::models as temperature_readings,
    tray_configurations::regions::models as regions,
    tray_configurations::trays::models as trays, tray_configurations::wells::models as wells,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, EntityTrait, QueryOrder, entity::prelude::*};
use uuid::Uuid;

// Helper function to load temperature readings and calculate time span
async fn load_temperature_data(
    experiment_id: Uuid,
    db: &impl ConnectionTrait,
) -> Result<
    (
        Vec<temperature_readings::Model>,
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
        temp_readings_data,
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
    let experiment_assets = crate::routes::assets::models::Entity::find()
        .filter(crate::routes::assets::models::Column::ExperimentId.eq(experiment_id))
        .filter(crate::routes::assets::models::Column::Type.eq("image"))
        .all(db)
        .await?;

    // Create filename-to-asset-id mapping (strip .jpg extension for matching)
    let filename_to_asset_id: std::collections::HashMap<String, Uuid> = experiment_assets
        .iter()
        .filter_map(|asset| {
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
            Some((filename_without_ext, asset.id))
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
        std::collections::HashMap<Uuid, i32>,
        std::collections::HashSet<Uuid>,
        usize,
        usize,
        usize,
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

    Ok((
        phase_transitions_data,
        well_final_states,
        wells_with_transitions,
        wells_with_data,
        wells_frozen,
        wells_liquid,
    ))
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
            crate::routes::treatments::models::Treatment,
            Option<crate::routes::samples::models::Sample>,
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

    use crate::routes::{
        locations::models as locations, samples::models as samples,
        treatments::models as treatments,
    };

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
                let sample_api: crate::routes::samples::models::Sample = (*sample).clone().into();
                sample_api
            })
        });

        let treatment_info: crate::routes::treatments::models::Treatment = treatment.clone().into();

        treatment_map.insert(treatment.id, (treatment_info, sample_info));
    }

    Ok(treatment_map)
}

// Helper function to build well summaries from loaded data
fn build_well_summaries(
    experiment_wells: &[wells::Model],
    phase_transitions_data: &[(well_phase_transitions::Model, Option<wells::Model>)],
    temp_readings_map: &std::collections::HashMap<Uuid, temperature_readings::Model>,
    filename_to_asset_id: &std::collections::HashMap<String, Uuid>,
    first_timestamp: Option<DateTime<Utc>>,
    well_final_states: &std::collections::HashMap<Uuid, i32>,
    experiment_regions: &[regions::Model],
    treatment_map: &std::collections::HashMap<
        Uuid,
        (
            crate::routes::treatments::models::Treatment,
            Option<crate::routes::samples::models::Sample>,
        ),
    >,
    tray_map: &std::collections::HashMap<Uuid, trays::Model>,
) -> Result<Vec<WellSummary>, DbErr> {
    let mut well_summaries = Vec::new();

    for well in experiment_wells {
        // Get tray information for tray_name lookup later
        let tray_info = tray_map.get(&well.tray_id);
        
        // Coordinates are now stored directly as alphanumeric format
        // No transformation needed - wells store exactly what should be displayed
        let coordinate = format!("{}{}", well.row_letter, well.column_number);

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

        // Get temperature probe values and image filename at first phase change
        let temperature_and_image = first_phase_change_transition
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

                let temperature_probes = super::temperatures::models::TemperatureReading {
                    experiment_id: temp_reading.experiment_id,
                    timestamp: temp_reading.timestamp,
                    id: temp_reading.id,
                    image_filename: temp_reading.image_filename.clone(),
                    created_at: temp_reading.created_at,
                    probe_1: temp_reading.probe_1.map(|d| d.round_dp(3)),
                    probe_2: temp_reading.probe_2.map(|d| d.round_dp(3)),
                    probe_3: temp_reading.probe_3.map(|d| d.round_dp(3)),
                    probe_4: temp_reading.probe_4.map(|d| d.round_dp(3)),
                    probe_5: temp_reading.probe_5.map(|d| d.round_dp(3)),
                    probe_6: temp_reading.probe_6.map(|d| d.round_dp(3)),
                    probe_7: temp_reading.probe_7.map(|d| d.round_dp(3)),
                    probe_8: temp_reading.probe_8.map(|d| d.round_dp(3)),
                    average,
                };

                (temperature_probes, temp_reading.image_filename.clone())
            });

        let first_phase_change_temperature_probes = temperature_and_image
            .as_ref()
            .map(|(temp_probes, _)| temp_probes.clone());
        let image_filename_at_freeze = temperature_and_image
            .as_ref()
            .and_then(|(_, image_filename)| image_filename.clone());

        // Look up asset ID for this image filename
        let image_asset_id = image_filename_at_freeze
            .as_ref()
            .and_then(|filename| filename_to_asset_id.get(filename))
            .copied();

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
        // Convert row letter to 0-based index (A=0, B=1, etc.)
        let well_row_0based = well.row_letter.chars().next()
            .map(|c| (c as u8 - b'A') as i32)
            .unwrap_or(0);
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

        // Get treatment and sample info if region exists
        let (_treatment, sample) = region
            .and_then(|r| r.treatment_id)
            .and_then(|treatment_id| treatment_map.get(&treatment_id))
            .map_or((None, None), |(t, s)| (Some(t.clone()), s.clone()));

        // Get tray information
        let tray_info = tray_map.get(&well.tray_id);
        let tray_name = tray_info.and_then(|t| t.name.clone());

        let well_summary = WellSummary {
            row_letter: well.row_letter.clone(),
            column_number: well.column_number,
            coordinate,
            first_phase_change_time,
            first_phase_change_seconds,
            first_phase_change_temperature_probes,
            final_state,
            image_filename_at_freeze,
            image_asset_id,
            // sample_id: sample.as_ref().map(|s| s.id), // ID for linking to /samples/{id}
            // treatment_id: treatment.as_ref().map(|t| t.id), // ID for linking to /treatments/{id}
            tray_id: Some(well.tray_id.to_string()),
            tray_name,
            dilution_factor: region.and_then(|r| r.dilution_factor),
            // treatment: None,
            sample,
        };

        well_summaries.push(well_summary);
    }

    Ok(well_summaries)
}

// Generate experiment results summary (now simplified to orchestrate helper functions)
pub(super) async fn build_results_summary(
    experiment_id: Uuid,
    db: &impl ConnectionTrait,
) -> Result<Option<ExperimentResultsSummary>, DbErr> {
    // Load all required data using helper functions
    let (
        _temp_readings_data,
        temp_readings_map,
        first_timestamp,
        last_timestamp,
        total_time_points,
    ) = load_temperature_data(experiment_id, db).await?;

    let filename_to_asset_id = load_experiment_assets(experiment_id, db).await?;

    let experiment_regions = regions::Entity::find()
        .filter(regions::Column::ExperimentId.eq(experiment_id))
        .all(db)
        .await?;

    let (
        phase_transitions_data,
        well_final_states,
        wells_with_transitions,
        wells_with_data,
        wells_frozen,
        wells_liquid,
    ) = process_phase_transitions(experiment_id, db).await?;

    let (experiment_wells, tray_map) = load_experiment_wells_and_trays(
        experiment_id,
        &wells_with_transitions,
        &phase_transitions_data,
        db,
    )
    .await?;

    let treatment_map = load_treatment_and_sample_data(&experiment_regions, db).await?;

    // Build well summaries using helper function
    let well_summaries = build_well_summaries(
        &experiment_wells,
        &phase_transitions_data,
        &temp_readings_map,
        &filename_to_asset_id,
        first_timestamp,
        &well_final_states,
        &experiment_regions,
        &treatment_map,
        &tray_map,
    )?;

    // Build hierarchical sample results from well summaries
    let sample_results = build_sample_results_from_wells(&well_summaries);

    // Always return a summary, even if empty
    Ok(Some(ExperimentResultsSummary {
        total_wells: experiment_wells.len(),
        wells_with_data,
        wells_frozen,
        wells_liquid,
        total_time_points,
        first_timestamp,
        last_timestamp,
        sample_results,
    }))
}

/// Build hierarchical sample results from flat well summaries
fn build_sample_results_from_wells(well_summaries: &[WellSummary]) -> Vec<SampleResultsSummary> {
    use std::collections::HashMap;
    
    // Group wells by sample, then by treatment
    let mut sample_map: HashMap<String, HashMap<String, Vec<WellSummary>>> = HashMap::new();
    
    for well in well_summaries {
        // Use sample info if available, otherwise create placeholder
        let sample_key = well.sample.as_ref()
            .map(|s| s.id.to_string())
            .unwrap_or_else(|| "unknown_sample".to_string());
        
        // For treatment, we need to extract from the well somehow
        // Since WellSummary doesn't directly have treatment info, we'll need to group by sample only for now
        let treatment_key = "default_treatment".to_string();
        
        sample_map.entry(sample_key)
            .or_insert_with(HashMap::new)
            .entry(treatment_key)
            .or_insert_with(Vec::new)
            .push(well.clone());
    }
    
    // Convert to the expected structure
    let mut sample_results = Vec::new();
    
    for (sample_key, treatment_map) in sample_map {
        // Get sample info from first well
        let sample_info = well_summaries.iter()
            .find(|w| w.sample.as_ref().map(|s| s.id.to_string()).unwrap_or_else(|| "unknown_sample".to_string()) == sample_key)
            .and_then(|w| w.sample.clone());
        
        if let Some(sample) = sample_info {
            let mut treatments = Vec::new();
            
            for (_, wells) in treatment_map {
                // Count frozen and liquid wells
                let wells_frozen = wells.iter().filter(|w| w.final_state.as_ref().map(|s| s == "frozen").unwrap_or(false)).count();
                let wells_liquid = wells.iter().filter(|w| w.final_state.as_ref().map(|s| s == "liquid").unwrap_or(false)).count();
                
                // Create a placeholder treatment since we don't have treatment info in WellSummary
                let treatment = crate::routes::treatments::models::Treatment {
                    id: uuid::Uuid::new_v4(),
                    name: crate::routes::treatments::models::TreatmentName::None,
                    notes: None,
                    sample_id: Some(sample.id),
                    created_at: chrono::Utc::now(),
                    last_updated: chrono::Utc::now(),
                    enzyme_volume_litres: None,
                    experimental_results: vec![],
                    statistics: None,
                    dilution_summaries: vec![],
                };
                
                let treatment_summary = TreatmentResultsSummary {
                    treatment,
                    wells,
                    wells_frozen,
                    wells_liquid,
                };
                
                treatments.push(treatment_summary);
            }
            
            let sample_summary = SampleResultsSummary {
                sample,
                treatments,
            };
            
            sample_results.push(sample_summary);
        }
    }
    
    sample_results
}




