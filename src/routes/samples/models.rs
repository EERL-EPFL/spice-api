use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel, traits::MergeIntoActiveModel};
use sea_orm::{
    ActiveModelTrait, ActiveValue, Condition, DatabaseConnection, DbErr, EntityTrait, Order,
    QueryOrder, QuerySelect, Set, entity::prelude::*,
};
use serde::{Deserialize, Serialize};
use spice_entity::samples::Model;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct SampleTreatment {
    pub id: Uuid,
    pub name: Option<String>,
    pub notes: Option<String>,
    pub enzyme_volume_litres: Option<Decimal>,
}
impl From<spice_entity::treatments::Model> for SampleTreatment {
    fn from(model: spice_entity::treatments::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            notes: model.notes,
            enzyme_volume_litres: model.enzyme_volume_litres,
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "spice_entity::samples::ActiveModel"]
pub struct Sample {
    #[crudcrate(update_model = false, create_model = false, on_create = Uuid::new_v4())]
    id: Uuid,
    name: String,
    r#type: spice_entity::SampleType,
    material_description: Option<String>,
    extraction_procedure: Option<String>,
    filter_substrate: Option<String>,
    suspension_volume_liters: Option<Decimal>,
    air_volume_liters: Option<Decimal>,
    water_volume_liters: Option<Decimal>,
    initial_concentration_gram_l: Option<Decimal>,
    well_volume_liters: Option<Decimal>,
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
            suspension_volume_liters: model.suspension_volume_liters,
            air_volume_liters: model.air_volume_liters,
            water_volume_liters: model.water_volume_liters,
            initial_concentration_gram_l: model.initial_concentration_gram_l,
            well_volume_liters: model.well_volume_liters,
            remarks: model.remarks,
            created_at: model.created_at.into(),
            last_updated: model.last_updated.into(),
            flow_litres_per_minute: model.flow_litres_per_minute,
            total_volume: model.total_volume,
            location_id: model.location_id,
            latitude: model.latitude,
            longitude: model.longitude,
            treatments: vec![],
        }
    }
}

#[async_trait]
impl CRUDResource for Sample {
    type EntityType = spice_entity::samples::Entity;
    type ColumnType = spice_entity::samples::Column;
    type ActiveModelType = spice_entity::samples::ActiveModel;
    type CreateModel = SampleCreate;
    type UpdateModel = SampleUpdate;

    const ID_COLUMN: Self::ColumnType = spice_entity::samples::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "samples";
    const RESOURCE_NAME_SINGULAR: &'static str = "sample";
    const RESOURCE_DESCRIPTION: &'static str =
        "This resource manages samples associated with experiments.";

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

        let treatments_vec = models
            .load_many(spice_entity::treatments::Entity, db)
            .await?;

        let mut models: Vec<Self> = models.into_iter().map(Self::from).collect();
        for (i, model) in models.iter_mut().enumerate() {
            // treatments_vec[i] is Vec<spice_entity::treatments::Model>
            model.treatments = treatments_vec[i]
                .iter()
                .cloned()
                .map(SampleTreatment::from)
                .collect();
        }
        if models.is_empty() {
            return Err(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_PLURAL
            )));
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
            .find_related(spice_entity::treatments::Entity)
            .all(db)
            .await?;

        let mut model: Self = model.into();
        model.treatments = treatments.into_iter().map(SampleTreatment::from).collect();

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

        let updated_model = update_data.merge_into_activemodel(existing);
        let _updated = updated_model.update(db).await?;

        // Update treatments if provided
        if let Some(treatments) = treatments {
            // Fetch existing treatments for this sample
            let existing_treatments = spice_entity::treatments::Entity::find()
                .filter(spice_entity::treatments::Column::SampleId.eq(id))
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
                    let mut active_treatment: spice_entity::treatments::ActiveModel =
                        existing.clone().into();
                    active_treatment.name = Set(treatment.name.clone());
                    active_treatment.notes = Set(treatment.notes.clone());
                    active_treatment.enzyme_volume_litres = Set(treatment.enzyme_volume_litres);
                    // sample_id should always be set
                    active_treatment.sample_id = Set(Some(id));
                    let _ = active_treatment.update(db).await?;
                } else {
                    // Insert new treatment
                    let active_treatment = spice_entity::treatments::ActiveModel {
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
                    let _ = spice_entity::treatments::Entity::delete_by_id(*existing_id)
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
        mut create_data: Self::CreateModel,
    ) -> Result<Self, DbErr> {
        // Extract treatments if present
        let treatments = if create_data.treatments.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut create_data.treatments))
        };

        let active_model: Self::ActiveModelType = create_data.into();
        let inserted = active_model.insert(db).await?;
        println!("Inserted sample with ID: {}", inserted.id);
        let sample_id = inserted.id;
        // Insert treatments if provided
        if let Some(treatments) = treatments {
            for treatment in treatments {
                let active_treatment = spice_entity::treatments::ActiveModel {
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
                "suspension_volume_liters",
                Self::ColumnType::SuspensionVolumeLiters,
            ),
            ("air_volume_liters", Self::ColumnType::AirVolumeLiters),
            ("water_volume_liters", Self::ColumnType::WaterVolumeLiters),
            (
                "initial_concentration_gram_l",
                Self::ColumnType::InitialConcentrationGramL,
            ),
            ("well_volume_liters", Self::ColumnType::WellVolumeLiters),
            ("created_at", Self::ColumnType::CreatedAt),
            ("last_updated", Self::ColumnType::LastUpdated),
            ("location_id", Self::ColumnType::LocationId),
            ("remarks", Self::ColumnType::Remarks),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![("name", Self::ColumnType::Name)]
    }
}
