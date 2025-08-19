use crate::routes::{
    experiments::{
        models as experiments, phase_transitions::models as well_phase_transitions,
        temperatures::models as temperature_readings,
    },
    tray_configurations::{regions::models as regions, wells::models as wells},
    treatments::models as treatments,
};
use crate::routes::{
    nucleation_events::models::{NucleationEvent, NucleationStatistics},
    treatments::views::Treatment,
};
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, EntityTrait, entity::prelude::*};
use uuid::Uuid;

/// Fetch all experimental results for a specific sample across all experiments
#[allow(clippy::too_many_lines)]
pub(super) async fn fetch_experimental_results_for_sample(
    db: &DatabaseConnection,
    sample_id: Uuid,
) -> Result<Vec<NucleationEvent>, DbErr> {
    // Find all treatments for this sample
    let sample_treatments = treatments::Entity::find()
        .filter(treatments::Column::SampleId.eq(sample_id))
        .all(db)
        .await?;

    let treatment_ids: Vec<Uuid> = sample_treatments.iter().map(|t| t.id).collect();
    if treatment_ids.is_empty() {
        return Ok(vec![]);
    }

    // Find all regions that use any of these treatments
    let regions_data = regions::Entity::find()
        .filter(regions::Column::TreatmentId.is_in(treatment_ids))
        .find_with_related(experiments::Entity)
        .all(db)
        .await?;

    let mut nucleation_events = Vec::new();

    for (region, experiments_list) in regions_data {
        for experiment in experiments_list {
            // Get phase transitions for this experiment
            let phase_transitions_data = well_phase_transitions::Entity::find()
                .filter(well_phase_transitions::Column::ExperimentId.eq(experiment.id))
                .find_also_related(wells::Entity)
                .all(db)
                .await?;

            // Get temperature readings for this experiment
            let temp_readings_data = temperature_readings::Entity::find()
                .filter(temperature_readings::Column::ExperimentId.eq(experiment.id))
                .all(db)
                .await?;

            let temp_readings_map: std::collections::HashMap<Uuid, &temperature_readings::Model> =
                temp_readings_data.iter().map(|tr| (tr.id, tr)).collect();

            // Get tray information - region.tray_id is i32, but we need to find by sequence/order
            // For now, let's use the tray name from the region or a placeholder
            let tray_name = format!("P{}", region.tray_id.unwrap_or(1));

            // Process wells that fall within this region's coordinates
            for (transition, well_opt) in &phase_transitions_data {
                if let Some(well) = well_opt {
                    // Check if well is within region bounds
                    let well_in_region =
                        if let (Some(row_min), Some(row_max), Some(col_min), Some(col_max)) = (
                            region.row_min,
                            region.row_max,
                            region.col_min,
                            region.col_max,
                        ) {
                            well.row_letter.chars().next().map_or(0, |c| i32::from(c as u8 - b'A')) >= row_min
                                && well.row_letter.chars().next().map_or(0, |c| i32::from(c as u8 - b'A')) <= row_max
                                && well.column_number >= (col_min + 1)
                                && well.column_number <= (col_max + 1)
                        } else {
                            false
                        };

                    if !well_in_region {
                        continue;
                    }

                    // Only process first freezing event (0→1 transition)
                    if transition.previous_state != 0 || transition.new_state != 1 {
                        continue;
                    }

                    // Get temperature data at nucleation time
                    let temperature_avg = temp_readings_map
                        .get(&transition.temperature_reading_id)
                        .and_then(|temp_reading| {
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

                            let non_null_values: Vec<Decimal> =
                                probe_values.into_iter().flatten().collect();
                            if non_null_values.is_empty() {
                                None
                            } else {
                                let sum: Decimal = non_null_values.iter().sum();
                                Some(sum / Decimal::from(non_null_values.len()))
                            }
                        });

                    // Calculate time from experiment start
                    let nucleation_time_seconds = temp_readings_data
                        .first()
                        .map(|tr| tr.timestamp)
                        .map(|start_time| (transition.timestamp - start_time).num_seconds());

                    // Well coordinates are already in alphanumeric format
                    let well_coordinate = format!("{}{}", well.row_letter, well.column_number);

                    // Find the treatment for this region
                    let treatment = sample_treatments
                        .iter()
                        .find(|t| t.id == region.treatment_id.unwrap_or_default());

                    let nucleation_event = NucleationEvent {
                        experiment_id: experiment.id,
                        experiment_name: experiment.name.clone(),
                        experiment_date: experiment.performed_at,
                        well_coordinate,
                        tray_name: Some(tray_name.clone()),
                        nucleation_time_seconds,
                        nucleation_temperature_avg_celsius: temperature_avg,
                        freezing_time_seconds: nucleation_time_seconds, // UI compatibility
                        freezing_temperature_avg: temperature_avg,      // UI compatibility
                        dilution_factor: region.dilution_factor,
                        final_state: "frozen".to_string(), // Since this is a 0→1 transition
                        treatment_id: treatment.map(|t| t.id),
                        treatment_name: treatment.map(|t| format!("{:?}", t.name)), // Convert enum to string
                    };

                    nucleation_events.push(nucleation_event);
                }
            }
        }
    }

    Ok(nucleation_events)
}

/// Convert treatment model to `TreatmentWithResults` by fetching experimental data
pub(super) async fn treatment_to_treatment_with_results(
    treatment: crate::routes::treatments::models::Model,
    sample_id: Uuid,
    db: &DatabaseConnection,
) -> Result<Treatment, DbErr> {
    // Fetch all experimental results for the sample, then filter by this treatment
    let all_results = fetch_experimental_results_for_sample(db, sample_id).await?;

    // Filter results to only include this specific treatment
    // We need to check which experiments used this treatment through regions
    let experimental_results = filter_results_by_treatment(db, all_results, treatment.id).await?;

    // Calculate statistics from the filtered results
    let statistics = NucleationStatistics::from_events(&experimental_results);

    // Calculate dilution summaries
    let dilution_summaries =
        NucleationStatistics::dilution_summaries_from_events(&experimental_results);

    Ok(Treatment {
        id: treatment.id,
        sample_id: Some(sample_id),
        created_at: treatment.created_at,
        last_updated: treatment.last_updated,
        name: treatment.name,
        notes: treatment.notes,
        enzyme_volume_litres: treatment.enzyme_volume_litres,
        experimental_results,
        statistics,
        dilution_summaries,
    })
}

/// Filter nucleation events to only include those from experiments that used the specified treatment
async fn filter_results_by_treatment(
    db: &DatabaseConnection,
    all_results: Vec<NucleationEvent>,
    treatment_id: Uuid,
) -> Result<Vec<NucleationEvent>, DbErr> {
    use crate::routes::tray_configurations::regions::models as regions;

    // Get all regions that use this treatment
    let treatment_regions = regions::Entity::find()
        .filter(regions::Column::TreatmentId.eq(treatment_id))
        .all(db)
        .await?;

    let treatment_experiment_ids: std::collections::HashSet<Uuid> = treatment_regions
        .iter()
        .map(|r| r.experiment_id) // experiment_id is required, not optional
        .collect();

    // Filter results to only include experiments that used this treatment
    let filtered_results = all_results
        .into_iter()
        .filter(|result| treatment_experiment_ids.contains(&result.experiment_id))
        .collect();

    Ok(filtered_results)
}
