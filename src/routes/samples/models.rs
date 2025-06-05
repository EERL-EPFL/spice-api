use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel};
use sea_orm::{
    ActiveValue, Condition, DatabaseConnection, EntityTrait, Order, QueryOrder, QuerySelect,
    entity::prelude::*,
};
use serde::{Deserialize, Serialize};
use spice_entity::samples::Model;
use utoipa::ToSchema;
use uuid::Uuid;

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
    background_region_key: Option<String>,
    remarks: Option<String>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    last_updated: DateTime<Utc>,
    campaign_id: Option<Uuid>,
    latitude: Option<Decimal>,
    longitude: Option<Decimal>,
    start_time: Option<DateTime<Utc>>,
    stop_time: Option<DateTime<Utc>>,
    flow_litres_per_minute: Option<f64>,
    total_volume: Option<f64>,
}

impl From<Model> for Sample {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            r#type: model.r#type,
            // treatment: model.treatment,
            start_time: model.start_time,
            stop_time: model.stop_time,
            material_description: model.material_description,
            extraction_procedure: model.extraction_procedure,
            filter_substrate: model.filter_substrate,
            suspension_volume_liters: model.suspension_volume_liters,
            air_volume_liters: model.air_volume_liters,
            water_volume_liters: model.water_volume_liters,
            initial_concentration_gram_l: model.initial_concentration_gram_l,
            well_volume_liters: model.well_volume_liters,
            background_region_key: model.background_region_key,
            remarks: model.remarks,
            created_at: model.created_at,
            last_updated: model.last_updated,
            flow_litres_per_minute: model.flow_litres_per_minute,
            total_volume: model.total_volume,
            campaign_id: model.campaign_id,
            latitude: model.latitude,
            longitude: model.longitude,
        }
    }
}

#[async_trait]
impl CRUDResource for Sample {
    type EntityType = spice_entity::samples::Entity;
    type ColumnType = spice_entity::samples::Column;
    type ModelType = spice_entity::samples::Model;
    type ActiveModelType = spice_entity::samples::ActiveModel;
    type ApiModel = Sample;
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
    ) -> Result<Vec<Self::ApiModel>, DbErr> {
        let models = Self::EntityType::find()
            .filter(condition)
            .order_by(order_column, order_direction)
            .offset(offset)
            .limit(limit)
            .all(db)
            .await?;
        Ok(models.into_iter().map(Self::ApiModel::from).collect())
    }

    async fn get_one(db: &DatabaseConnection, id: Uuid) -> Result<Self::ApiModel, DbErr> {
        let model =
            Self::EntityType::find_by_id(id)
                .one(db)
                .await?
                .ok_or(DbErr::RecordNotFound(format!(
                    "{} not found",
                    Self::RESOURCE_NAME_SINGULAR
                )))?;
        Ok(Self::ApiModel::from(model))
    }

    async fn update(
        db: &DatabaseConnection,
        id: Uuid,
        update_data: Self::UpdateModel,
    ) -> Result<Self::ApiModel, DbErr> {
        let existing: Self::ActiveModelType = Self::EntityType::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_PLURAL
            )))?
            .into();

        let updated_model = update_data.merge_into_activemodel(existing);
        let updated = updated_model.update(db).await?;
        Ok(Self::ApiModel::from(updated))
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
            (
                "background_region_key",
                Self::ColumnType::BackgroundRegionKey,
            ),
            ("created_at", Self::ColumnType::CreatedAt),
            ("last_updated", Self::ColumnType::LastUpdated),
            ("campaign_id", Self::ColumnType::CampaignId),
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
            (
                "background_region_key",
                Self::ColumnType::BackgroundRegionKey,
            ),
            ("created_at", Self::ColumnType::CreatedAt),
            ("remarks", Self::ColumnType::Remarks),
        ]
    }
}
