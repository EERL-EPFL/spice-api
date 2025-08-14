use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels};
use rust_decimal::Decimal;
use sea_orm::{EntityTrait, entity::prelude::*};
use uuid::Uuid;
use crate::routes::nucleation_events::models::{NucleationEvent, NucleationStatistics};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "treatments")]
#[crudcrate(
    generate_router,
    api_struct = "Treatment",
    name_singular = "treatment",
    name_plural = "treatments",
    description = "Treatments are applied to samples during experiments to study their effects on ice nucleation.",
    fn_get_one = get_one_treatment,
)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[crudcrate(sortable, filterable, enum_field)]
    pub name: TreatmentName,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub notes: Option<String>,
    #[crudcrate(sortable, filterable, list_model = false)]
    pub sample_id: Option<Uuid>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub last_updated: DateTime<Utc>,
    #[sea_orm(column_type = "Decimal(Some((16, 10)))", nullable)]
    #[crudcrate(sortable, filterable)]
    pub enzyme_volume_litres: Option<Decimal>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], list_model = false, create_model = false, update_model = false)]
    pub experimental_results: Vec<NucleationEvent>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = NucleationStatistics::default(), list_model = false, create_model = false, update_model = false)]
    pub statistics: NucleationStatistics,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::routes::tray_configurations::regions::models::Entity")]
    Regions,
    #[sea_orm(
        belongs_to = "crate::routes::samples::models::Entity",
        from = "Column::SampleId",
        to = "crate::routes::samples::models::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Samples,
}

impl Related<crate::routes::tray_configurations::regions::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Regions.def()
    }
}

impl Related<crate::routes::samples::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Samples.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, ToSchema, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "treatment_name")]
pub enum TreatmentName {
    #[sea_orm(string_value = "none")]
    #[serde(rename = "none")]
    None,
    #[sea_orm(string_value = "heat")]
    #[serde(rename = "heat")]
    Heat,
    #[sea_orm(string_value = "h2o2")]
    #[serde(rename = "h2o2")]
    H2o2,
}

// Experimental results functionality - not implemented yet
/// Fetch all experimental results for a specific treatment across all experiments
async fn fetch_experimental_results_for_treatment(
    db: &DatabaseConnection,
    treatment_id: Uuid,
) -> Result<Vec<NucleationEvent>, DbErr> {
    use crate::routes::{
        experiments::{
            models as experiments, phase_transitions::models as well_phase_transitions,
            temperatures::models as temperature_readings,
        },
        tray_configurations::{
            regions::models as regions,
            wells::models as wells,
        },
    };

    // Get the treatment information
    let treatment = Entity::find_by_id(treatment_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Treatment not found".to_string()))?;

    // Find all regions that use this treatment
    let regions_data = regions::Entity::find()
        .filter(regions::Column::TreatmentId.eq(treatment_id))
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
                    let well_in_region = if let (Some(row_min), Some(row_max), Some(col_min), Some(col_max)) = (
                        region.row_min,
                        region.row_max,
                        region.col_min,
                        region.col_max,
                    ) {
                        well.row_number >= (row_min + 1) &&
                        well.row_number <= (row_max + 1) &&
                        well.column_number >= (col_min + 1) &&
                        well.column_number <= (col_max + 1)
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

                            let non_null_values: Vec<Decimal> = probe_values.into_iter().flatten().collect();
                            if non_null_values.is_empty() {
                                None
                            } else {
                                let sum: Decimal = non_null_values.iter().sum();
                                Some(sum / Decimal::from(non_null_values.len()))
                            }
                        });

                    // Calculate time from experiment start
                    let nucleation_time_seconds = temp_readings_data.first().map(|tr| tr.timestamp).map(|start_time| (transition.timestamp - start_time).num_seconds());

                    // Convert well coordinates to string format (A1, B2, etc.)
                    // Row determines letter (A, B, C...), Column determines number (1, 2, 3...)
                    let well_coordinate = format!(
                        "{}{}",
                        char::from(b'A' + u8::try_from(well.row_number - 1).unwrap_or(0)),
                        well.column_number
                    );

                    let nucleation_event = NucleationEvent {
                        experiment_id: experiment.id,
                        experiment_name: experiment.name.clone(),
                        experiment_date: experiment.performed_at,
                        well_coordinate,
                        tray_name: Some(tray_name.clone()),
                        nucleation_time_seconds,
                        nucleation_temperature_avg_celsius: temperature_avg,
                        freezing_time_seconds: nucleation_time_seconds, // UI compatibility
                        freezing_temperature_avg: temperature_avg, // UI compatibility
                        dilution_factor: region.dilution_factor,
                        final_state: "frozen".to_string(), // Since this is a 0→1 transition
                        treatment_id: Some(treatment_id),
                        treatment_name: Some(format!("{:?}", treatment.name)), // Convert enum to string
                    };

                    nucleation_events.push(nucleation_event);
                }
            }
        }
    }

    Ok(nucleation_events)
}

/// Custom `get_one` that loads experimental results and statistics
async fn get_one_treatment(db: &DatabaseConnection, id: Uuid) -> Result<Treatment, DbErr> {
    let model = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Treatment not found".to_string()))?;

    // Fetch experimental results for this treatment
    let experimental_results = fetch_experimental_results_for_treatment(db, id).await?;

    // Calculate statistics from the results
    let statistics = NucleationStatistics::from_events(&experimental_results);

    let mut treatment: Treatment = model.into();
    treatment.experimental_results = experimental_results;
    treatment.statistics = statistics;

    Ok(treatment)
}
