use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel, traits::MergeIntoActiveModel};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, Condition, DatabaseConnection, DbErr, EntityTrait, Order,
    QueryOrder, QuerySelect, Set, entity::prelude::*,
};
use serde::{Deserialize, Serialize};
use spice_entity::samples::Model;
use spice_entity::sea_orm_active_enums::TreatmentName;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct SampleTreatment {
    pub id: Uuid,
    pub name: TreatmentName,
    pub notes: Option<String>,
    pub enzyme_volume_litres: Option<Decimal>,
}

impl From<crate::routes::treatments::models::Model> for SampleTreatment {
    fn from(model: crate::routes::treatments::models::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            notes: model.notes,
            enzyme_volume_litres: model.enzyme_volume_litres,
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct SampleTreatmentCreate {
    pub name: TreatmentName,
    pub notes: Option<String>,
    pub enzyme_volume_litres: Option<Decimal>,
}

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

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct SampleCreateCustom {
    pub name: String,
    pub r#type: spice_entity::SampleType,
    pub material_description: Option<String>,
    pub extraction_procedure: Option<String>,
    pub filter_substrate: Option<String>,
    pub suspension_volume_litres: Option<Decimal>,
    pub air_volume_litres: Option<Decimal>,
    pub water_volume_litres: Option<Decimal>,
    pub initial_concentration_gram_l: Option<Decimal>,
    pub well_volume_litres: Option<Decimal>,
    pub remarks: Option<String>,
    pub location_id: Option<Uuid>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: Option<DateTime<Utc>>,
    pub flow_litres_per_minute: Option<Decimal>,
    pub total_volume: Option<Decimal>,
    #[serde(default)]
    pub treatments: Vec<SampleTreatmentCreate>,
}

impl From<SampleCreateCustom> for spice_entity::samples::ActiveModel {
    fn from(create: SampleCreateCustom) -> Self {
        spice_entity::samples::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            name: ActiveValue::Set(create.name),
            r#type: ActiveValue::Set(create.r#type),
            material_description: ActiveValue::Set(create.material_description),
            extraction_procedure: ActiveValue::Set(create.extraction_procedure),
            filter_substrate: ActiveValue::Set(create.filter_substrate),
            suspension_volume_litres: ActiveValue::Set(create.suspension_volume_litres),
            air_volume_litres: ActiveValue::Set(create.air_volume_litres),
            water_volume_litres: ActiveValue::Set(create.water_volume_litres),
            initial_concentration_gram_l: ActiveValue::Set(create.initial_concentration_gram_l),
            well_volume_litres: ActiveValue::Set(create.well_volume_litres),
            remarks: ActiveValue::Set(create.remarks),
            created_at: ActiveValue::Set(chrono::Utc::now().into()),
            last_updated: ActiveValue::Set(chrono::Utc::now().into()),
            location_id: ActiveValue::Set(create.location_id),
            latitude: ActiveValue::Set(create.latitude),
            longitude: ActiveValue::Set(create.longitude),
            start_time: ActiveValue::Set(create.start_time.map(std::convert::Into::into)),
            stop_time: ActiveValue::Set(create.stop_time.map(std::convert::Into::into)),
            flow_litres_per_minute: ActiveValue::Set(create.flow_litres_per_minute),
            total_volume: ActiveValue::Set(create.total_volume),
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "spice_entity::samples::ActiveModel"]
pub struct Sample {
    #[crudcrate(update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    name: String,
    r#type: spice_entity::SampleType,
    material_description: Option<String>,
    extraction_procedure: Option<String>,
    filter_substrate: Option<String>,
    suspension_volume_litres: Option<Decimal>,
    air_volume_litres: Option<Decimal>,
    water_volume_litres: Option<Decimal>,
    initial_concentration_gram_l: Option<Decimal>,
    well_volume_litres: Option<Decimal>,
    remarks: Option<String>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    last_updated: DateTime<Utc>,
    location_id: Option<Uuid>,
    latitude: Option<Decimal>,
    longitude: Option<Decimal>,
    start_time: Option<DateTime<Utc>>,
    stop_time: Option<DateTime<Utc>>,
    flow_litres_per_minute: Option<Decimal>,
    total_volume: Option<Decimal>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    pub treatments: Vec<SampleTreatment>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    pub experimental_results: Vec<ExperimentalResult>,
}

impl From<Model> for Sample {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            r#type: model.r#type,
            start_time: model.start_time.map(|dt| dt.with_timezone(&Utc)),
            stop_time: model.stop_time.map(|dt| dt.with_timezone(&Utc)),
            material_description: model.material_description,
            extraction_procedure: model.extraction_procedure,
            filter_substrate: model.filter_substrate,
            suspension_volume_litres: model.suspension_volume_litres,
            air_volume_litres: model.air_volume_litres,
            water_volume_litres: model.water_volume_litres,
            initial_concentration_gram_l: model.initial_concentration_gram_l,
            well_volume_litres: model.well_volume_litres,
            remarks: model.remarks,
            created_at: model.created_at.into(),
            last_updated: model.last_updated.into(),
            flow_litres_per_minute: model.flow_litres_per_minute,
            total_volume: model.total_volume,
            location_id: model.location_id,
            latitude: model.latitude,
            longitude: model.longitude,
            treatments: vec![],
            experimental_results: vec![],
        }
    }
}

// Helper function to fetch wells within region coordinates
async fn fetch_wells_in_region(
    db: &DatabaseConnection,
    region: &crate::routes::trays::regions::models::Model,
) -> Result<Vec<spice_entity::wells::Model>, DbErr> {
    if let (Some(row_min), Some(row_max), Some(col_min), Some(col_max)) = (
        region.row_min,
        region.row_max,
        region.col_min,
        region.col_max,
    ) {
        spice_entity::wells::Entity::find()
            .filter(
                spice_entity::wells::Column::RowNumber
                    .gte(row_min + 1) // Convert 0-based to 1-based
                    .and(spice_entity::wells::Column::RowNumber.lte(row_max + 1))
                    .and(spice_entity::wells::Column::ColumnNumber.gte(col_min + 1))
                    .and(spice_entity::wells::Column::ColumnNumber.lte(col_max + 1)),
            )
            .all(db)
            .await
    } else {
        Ok(vec![])
    }
}

// Helper function to calculate freezing time and temperature
async fn calculate_freezing_metrics(
    db: &DatabaseConnection,
    well_id: Uuid,
    experiment: &crate::routes::experiments::models::Model,
) -> Result<(Option<i64>, Option<Decimal>), DbErr> {
    let phase_transitions = spice_entity::well_phase_transitions::Entity::find()
        .filter(
            spice_entity::well_phase_transitions::Column::WellId
                .eq(well_id)
                .and(spice_entity::well_phase_transitions::Column::ExperimentId.eq(experiment.id))
                .and(spice_entity::well_phase_transitions::Column::PreviousState.eq(0))
                .and(spice_entity::well_phase_transitions::Column::NewState.eq(1)),
        )
        .find_with_related(crate::routes::experiments::temperatures::models::Entity)
        .all(db)
        .await?;

    if let Some((_transition, temp_readings)) = phase_transitions.first() {
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
                temp_reading.probe_1,
                temp_reading.probe_2,
                temp_reading.probe_3,
                temp_reading.probe_4,
                temp_reading.probe_5,
                temp_reading.probe_6,
                temp_reading.probe_7,
                temp_reading.probe_8,
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

        Ok((freezing_time, avg_temp))
    } else {
        Ok((None, None))
    }
}

// Helper function to determine final state of well
async fn determine_final_state(
    db: &DatabaseConnection,
    well_id: Uuid,
    experiment_id: Uuid,
) -> Result<String, DbErr> {
    let has_frozen_transition = spice_entity::well_phase_transitions::Entity::find()
        .filter(
            spice_entity::well_phase_transitions::Column::WellId
                .eq(well_id)
                .and(spice_entity::well_phase_transitions::Column::ExperimentId.eq(experiment_id))
                .and(spice_entity::well_phase_transitions::Column::NewState.eq(1)),
        )
        .one(db)
        .await?
        .is_some();

    Ok(if has_frozen_transition {
        "frozen".to_string()
    } else {
        "liquid".to_string()
    })
}

// Helper function to format well coordinate
fn format_well_coordinate(well: &spice_entity::wells::Model) -> String {
    format!(
        "{}{}",
        char::from(b'A' + u8::try_from(well.column_number - 1).unwrap_or(0)),
        well.row_number
    )
}

async fn fetch_experimental_results_for_sample(
    db: &DatabaseConnection,
    sample_id: Uuid,
) -> Result<Vec<ExperimentalResult>, DbErr> {
    // Find all treatments that use this sample
    let treatments = crate::routes::treatments::models::Entity::find()
        .filter(crate::routes::treatments::models::Column::SampleId.eq(sample_id))
        .all(db)
        .await?;

    if treatments.is_empty() {
        return Ok(vec![]);
    }

    let treatment_ids: Vec<Uuid> = treatments.iter().map(|t| t.id).collect();

    // Find all regions that use these treatments
    let regions = crate::routes::trays::regions::models::Entity::find()
        .filter(
            crate::routes::trays::regions::models::Column::TreatmentId.is_in(treatment_ids.clone()),
        )
        .find_with_related(crate::routes::experiments::models::Entity)
        .all(db)
        .await?;

    let mut experimental_results = Vec::new();

    // Iterate through each region and its associated experiments
    for (region, experiments) in regions {
        for experiment in experiments {
            let wells = fetch_wells_in_region(db, &region).await?;

            for well in wells {
                let (freezing_time_seconds, freezing_temperature_avg) =
                    calculate_freezing_metrics(db, well.id, &experiment).await?;

                let final_state = determine_final_state(db, well.id, experiment.id).await?;

                let well_coordinate = format_well_coordinate(&well);

                // Find the treatment for this region
                let treatment = treatments
                    .iter()
                    .find(|t| t.id == region.treatment_id.unwrap_or_default());

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
                    treatment_name: treatment.map(|t| format!("{:?}", t.name)),
                    treatment_id: treatment.map(|t| t.id),
                    dilution_factor: region.dilution_factor,
                    final_state,
                });
            }
        }
    }

    Ok(experimental_results)
}

#[async_trait]
impl CRUDResource for Sample {
    type EntityType = spice_entity::samples::Entity;
    type ColumnType = spice_entity::samples::Column;
    type ActiveModelType = spice_entity::samples::ActiveModel;
    type CreateModel = SampleCreateCustom;
    type UpdateModel = SampleUpdate;
    type ListModel = Self; // Use the same model for list view for now

    const ID_COLUMN: Self::ColumnType = spice_entity::samples::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "samples";
    const RESOURCE_NAME_SINGULAR: &'static str = "sample";
    const RESOURCE_DESCRIPTION: &'static str =
        "This resource manages samples associated with experiments.";

    async fn get_all(
        db: &DatabaseConnection,
        condition: &Condition,
        order_column: Self::ColumnType,
        order_direction: Order,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Self>, DbErr> {
        let models = Self::EntityType::find()
            .filter(condition.clone())
            .order_by(order_column, order_direction)
            .offset(offset)
            .limit(limit)
            .all(db)
            .await?;

        let treatments_vec = models
            .load_many(crate::routes::treatments::models::Entity, db)
            .await?;

        let mut models: Vec<Self> = models.into_iter().map(Self::from).collect();
        for (i, model) in models.iter_mut().enumerate() {
            // treatments_vec[i] is Vec<crate::routes::treatments::models::Model>
            model.treatments = treatments_vec[i]
                .iter()
                .cloned()
                .map(SampleTreatment::from)
                .collect();

            // Fetch experimental results for each sample
            model.experimental_results =
                fetch_experimental_results_for_sample(db, model.id).await?;
        }
        if models.is_empty() {
            return Ok(vec![]);
        }
        Ok(models)
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

        let treatments = model
            .find_related(crate::routes::treatments::models::Entity)
            .all(db)
            .await?;

        let experimental_results = fetch_experimental_results_for_sample(db, id).await?;

        let mut model: Self = model.into();
        model.treatments = treatments.into_iter().map(SampleTreatment::from).collect();
        model.experimental_results = experimental_results;

        Ok(model)
    }

    async fn update(
        db: &DatabaseConnection,
        id: Uuid,
        mut update_data: Self::UpdateModel,
    ) -> Result<Self, DbErr> {
        // Extract treatments if present
        let treatments = if update_data.treatments.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut update_data.treatments))
        };

        let existing: Self::ActiveModelType = Self::EntityType::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_PLURAL
            )))?
            .into();

        let updated_model = update_data.merge_into_activemodel(existing)?;
        let _updated = updated_model.update(db).await?;

        // Update treatments if provided
        if let Some(treatments) = treatments {
            // Fetch existing treatments for this sample
            let existing_treatments = crate::routes::treatments::models::Entity::find()
                .filter(crate::routes::treatments::models::Column::SampleId.eq(id))
                .all(db)
                .await?;

            let mut existing_map = std::collections::HashMap::new();
            for t in existing_treatments {
                existing_map.insert(t.id, t);
            }

            let mut incoming_ids = std::collections::HashSet::new();

            // Upsert incoming treatments
            for treatment in &treatments {
                incoming_ids.insert(treatment.id);

                if let Some(existing) = existing_map.get(&treatment.id) {
                    // Update existing treatment if any field changed
                    let mut active_treatment: crate::routes::treatments::models::ActiveModel =
                        existing.clone().into();
                    active_treatment.name = Set(treatment.name.clone());
                    active_treatment.notes = Set(treatment.notes.clone());
                    active_treatment.enzyme_volume_litres = Set(treatment.enzyme_volume_litres);
                    // sample_id should always be set
                    active_treatment.sample_id = Set(Some(id));
                    let _ = active_treatment.update(db).await?;
                } else {
                    // Insert new treatment
                    let active_treatment = crate::routes::treatments::models::ActiveModel {
                        id: Set(treatment.id),
                        sample_id: Set(Some(id)),
                        name: Set(treatment.name.clone()),
                        notes: Set(treatment.notes.clone()),
                        enzyme_volume_litres: Set(treatment.enzyme_volume_litres),
                        ..Default::default()
                    };
                    let _ = active_treatment.insert(db).await?;
                }
            }

            // Remove treatments that are no longer present
            for existing_id in existing_map.keys() {
                if !incoming_ids.contains(existing_id) {
                    let _ = crate::routes::treatments::models::Entity::delete_by_id(*existing_id)
                        .exec(db)
                        .await?;
                }
            }
        }

        // Reload with treatments
        Self::get_one(db, id).await
    }

    async fn create(
        db: &DatabaseConnection,
        create_data: Self::CreateModel,
    ) -> Result<Self, DbErr> {
        // Extract treatments if present
        let treatments = if create_data.treatments.is_empty() {
            None
        } else {
            Some(create_data.treatments.clone())
        };

        let active_model: Self::ActiveModelType = create_data.into();
        let inserted = active_model.insert(db).await?;
        println!("Inserted sample with ID: {}", inserted.id);
        let sample_id = inserted.id;

        // Insert treatments if provided
        if let Some(treatments) = treatments {
            for treatment in treatments {
                let active_treatment = crate::routes::treatments::models::ActiveModel {
                    id: ActiveValue::Set(uuid::Uuid::new_v4()),
                    sample_id: ActiveValue::Set(Some(sample_id)),
                    name: ActiveValue::Set(treatment.name),
                    notes: ActiveValue::Set(treatment.notes),
                    enzyme_volume_litres: ActiveValue::Set(treatment.enzyme_volume_litres),
                    ..Default::default()
                };
                let _ = active_treatment.insert(db).await?;
            }
        }

        // Reload with treatments
        Self::get_one(db, inserted.id).await
    }

    fn sortable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("id", Self::ColumnType::Id),
            ("name", Self::ColumnType::Name),
            ("type", Self::ColumnType::Type),
            (
                "material_description",
                Self::ColumnType::MaterialDescription,
            ),
            (
                "extraction_procedure",
                Self::ColumnType::ExtractionProcedure,
            ),
            ("filter_substrate", Self::ColumnType::FilterSubstrate),
            (
                "suspension_volume_litres",
                Self::ColumnType::SuspensionVolumeLitres,
            ),
            ("air_volume_litres", Self::ColumnType::AirVolumeLitres),
            ("water_volume_litres", Self::ColumnType::WaterVolumeLitres),
            (
                "initial_concentration_gram_l",
                Self::ColumnType::InitialConcentrationGramL,
            ),
            ("well_volume_litres", Self::ColumnType::WellVolumeLitres),
            ("created_at", Self::ColumnType::CreatedAt),
            ("last_updated", Self::ColumnType::LastUpdated),
            ("location_id", Self::ColumnType::LocationId),
            ("remarks", Self::ColumnType::Remarks),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("name", Self::ColumnType::Name),
            ("type", Self::ColumnType::Type),
            (
                "material_description",
                Self::ColumnType::MaterialDescription,
            ),
            (
                "extraction_procedure",
                Self::ColumnType::ExtractionProcedure,
            ),
            ("filter_substrate", Self::ColumnType::FilterSubstrate),
            (
                "suspension_volume_litres",
                Self::ColumnType::SuspensionVolumeLitres,
            ),
            ("air_volume_litres", Self::ColumnType::AirVolumeLitres),
            ("water_volume_litres", Self::ColumnType::WaterVolumeLitres),
            (
                "initial_concentration_gram_l",
                Self::ColumnType::InitialConcentrationGramL,
            ),
            ("well_volume_litres", Self::ColumnType::WellVolumeLitres),
            ("location_id", Self::ColumnType::LocationId),
        ]
    }
}
