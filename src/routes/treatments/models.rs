use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel};
use rust_decimal::Decimal;
use sea_orm::{ActiveValue, entity::prelude::*, DatabaseConnection, DbErr, EntityTrait};
use serde::{Deserialize, Serialize};
use spice_entity::sea_orm_active_enums::TreatmentName;
use spice_entity::treatments::Model;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct ExperimentalResult {
    pub experiment_id: Uuid,
    pub experiment_name: String,
    pub experiment_date: Option<DateTime<Utc>>,
    pub well_coordinate: String,
    pub tray_name: Option<String>,
    pub freezing_temperature_avg: Option<Decimal>,
    pub freezing_time_seconds: Option<i64>,
    pub treatment_name: Option<String>,
    pub treatment_id: Option<Uuid>,
    pub dilution_factor: Option<i32>,
    pub final_state: String,
}

#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "spice_entity::treatments::ActiveModel"]
pub struct Treatment {
    #[crudcrate(update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    name: TreatmentName,
    notes: Option<String>,
    sample_id: Option<Uuid>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    last_updated: DateTime<Utc>,
    enzyme_volume_litres: Option<Decimal>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    pub experimental_results: Vec<ExperimentalResult>,
}

impl From<Model> for Treatment {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            created_at: model.created_at.into(),
            last_updated: model.last_updated.into(),
            notes: model.notes,
            sample_id: model.sample_id,
            enzyme_volume_litres: model.enzyme_volume_litres,
            experimental_results: vec![],
        }
    }
}

async fn fetch_experimental_results_for_treatment(
    db: &DatabaseConnection,
    treatment_id: Uuid,
) -> Result<Vec<ExperimentalResult>, DbErr> {
    // Find all regions that use this treatment
    let regions = spice_entity::regions::Entity::find()
        .filter(spice_entity::regions::Column::TreatmentId.eq(treatment_id))
        .find_with_related(spice_entity::experiments::Entity)
        .all(db)
        .await?;

    let mut experimental_results = Vec::new();

    for (region, experiments) in regions {
        for experiment in experiments {
            // Find wells that fall within this region's coordinates
            let wells = if let (Some(row_min), Some(row_max), Some(col_min), Some(col_max)) = 
                (region.row_min, region.row_max, region.col_min, region.col_max) {
                spice_entity::wells::Entity::find()
                    .filter(
                        spice_entity::wells::Column::RowNumber
                            .gte(row_min + 1) // Convert 0-based to 1-based
                            .and(spice_entity::wells::Column::RowNumber.lte(row_max + 1))
                            .and(spice_entity::wells::Column::ColumnNumber.gte(col_min + 1))
                            .and(spice_entity::wells::Column::ColumnNumber.lte(col_max + 1))
                    )
                    .all(db)
                    .await?
            } else {
                // Skip this region if coordinates are not complete
                continue;
            };

            for well in wells {
                // Find phase transitions for this well in this experiment
                let phase_transitions = spice_entity::well_phase_transitions::Entity::find()
                    .filter(
                        spice_entity::well_phase_transitions::Column::WellId
                            .eq(well.id)
                            .and(spice_entity::well_phase_transitions::Column::ExperimentId.eq(experiment.id))
                            .and(spice_entity::well_phase_transitions::Column::PreviousState.eq(0))
                            .and(spice_entity::well_phase_transitions::Column::NewState.eq(1))
                    )
                    .find_with_related(spice_entity::temperature_readings::Entity)
                    .all(db)
                    .await?;

                // Get the first freezing transition and its temperature data
                let (freezing_time_seconds, freezing_temperature_avg) = if let Some((_transition, temp_readings)) = phase_transitions.first() {
                    let freezing_time = if let Some(temp_reading) = temp_readings.first() {
                        if let Some(experiment_start) = experiment.performed_at {
                            let transition_time = temp_reading.timestamp;
                            Some((transition_time - experiment_start).num_seconds())
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let avg_temp = if let Some(temp_reading) = temp_readings.first() {
                        // Calculate average of all 8 temperature probes
                        let temps = vec![
                            temp_reading.probe_1, temp_reading.probe_2,
                            temp_reading.probe_3, temp_reading.probe_4,
                            temp_reading.probe_5, temp_reading.probe_6,
                            temp_reading.probe_7, temp_reading.probe_8,
                        ];
                        let valid_temps: Vec<Decimal> = temps.into_iter().flatten().collect();
                        if valid_temps.is_empty() {
                            None
                        } else {
                            Some(valid_temps.iter().sum::<Decimal>() / Decimal::from(valid_temps.len()))
                        }
                    } else {
                        None
                    };

                    (freezing_time, avg_temp)
                } else {
                    (None, None)
                };

                // Determine final state - check if there are any frozen transitions
                let has_frozen_transition = spice_entity::well_phase_transitions::Entity::find()
                    .filter(
                        spice_entity::well_phase_transitions::Column::WellId
                            .eq(well.id)
                            .and(spice_entity::well_phase_transitions::Column::ExperimentId.eq(experiment.id))
                            .and(spice_entity::well_phase_transitions::Column::NewState.eq(1))
                    )
                    .one(db)
                    .await?
                    .is_some();

                let final_state = if has_frozen_transition { "frozen".to_string() } else { "liquid".to_string() };

                // Get well coordinate in A1 format
                let well_coordinate = format!(
                    "{}{}",
                    char::from(b'A' + (well.column_number - 1) as u8),
                    well.row_number
                );

                // Get tray name
                let tray = spice_entity::trays::Entity::find_by_id(well.tray_id)
                    .one(db)
                    .await?;

                experimental_results.push(ExperimentalResult {
                    experiment_id: experiment.id,
                    experiment_name: experiment.name.clone(),
                    experiment_date: experiment.performed_at.map(|dt| dt.with_timezone(&Utc)),
                    well_coordinate,
                    tray_name: tray.and_then(|t| t.name),
                    freezing_temperature_avg,
                    freezing_time_seconds,
                    treatment_name: Some(format!("{:?}", region.treatment_id)), // Could be improved to get actual treatment name
                    treatment_id: Some(treatment_id),
                    dilution_factor: region.dilution_factor,
                    final_state,
                });
            }
        }
    }

    Ok(experimental_results)
}

#[async_trait]
impl CRUDResource for Treatment {
    type EntityType = spice_entity::treatments::Entity;
    type ColumnType = spice_entity::treatments::Column;
    type ActiveModelType = spice_entity::treatments::ActiveModel;
    type CreateModel = TreatmentCreate;
    type UpdateModel = TreatmentUpdate;

    const ID_COLUMN: Self::ColumnType = spice_entity::treatments::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "treatments";
    const RESOURCE_NAME_SINGULAR: &'static str = "treatment";
    const RESOURCE_DESCRIPTION: &'static str =
        "This resource manages treatments applied to samples.";

    fn sortable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("id", Self::ColumnType::Id),
            ("name", Self::ColumnType::Name),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("name", Self::ColumnType::Name),
            ("created_at", Self::ColumnType::CreatedAt),
            ("last_updated", Self::ColumnType::LastUpdated),
        ]
    }

    async fn get_one(db: &DatabaseConnection, id: Uuid) -> Result<Self, DbErr> {
        let model =
            Self::EntityType::find_by_id(id)
                .one(db)
                .await?
                .ok_or(DbErr::RecordNotFound(format!(
                    "{} not found",
                    Self::RESOURCE_NAME_SINGULAR
                )))?;

        let experimental_results = fetch_experimental_results_for_treatment(db, id).await?;

        let mut model: Self = model.into();
        model.experimental_results = experimental_results;

        Ok(model)
    }
}
