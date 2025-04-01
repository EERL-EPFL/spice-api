use super::db::Model;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel};
use sea_orm::{
    ActiveValue, Condition, DatabaseConnection, EntityTrait, FromQueryResult, Order, QueryOrder,
    QuerySelect, entity::prelude::*,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(
    ToSchema, Serialize, Deserialize, FromQueryResult, ToUpdateModel, ToCreateModel, Clone,
)]
#[active_model = "super::db::ActiveModel"]
pub struct Experiment {
    #[crudcrate(update_model = false, update_model = false, on_create = Uuid::new_v4())]
    id: Uuid,
    name: String,
    campaign_id: Option<Uuid>,
    created_by: Option<String>,
    experiment_date: Option<DateTime<Utc>>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    last_updated: DateTime<Utc>,
    temperature_ramp: Option<f64>,
    temperature_start: Option<f64>,
    temperature_end: Option<f64>,
    is_calibration: bool,
    remarks: Option<String>,
}

impl From<Model> for Experiment {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            campaign_id: model.campaign_id,
            created_by: model.created_by,
            experiment_date: model.experiment_date,
            created_at: model.created_at,
            last_updated: model.last_updated,
            temperature_ramp: model.temperature_ramp,
            temperature_start: model.temperature_start,
            temperature_end: model.temperature_end,
            is_calibration: model.is_calibration,
            remarks: model.remarks,
        }
    }
}

#[async_trait]
impl CRUDResource for Experiment {
    type EntityType = super::db::Entity;
    type ColumnType = super::db::Column;
    type ModelType = super::db::Model;
    type ActiveModelType = super::db::ActiveModel;
    type ApiModel = Experiment;
    type CreateModel = ExperimentCreate;
    type UpdateModel = ExperimentUpdate;

    const ID_COLUMN: Self::ColumnType = super::db::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "experiments";
    const RESOURCE_NAME_SINGULAR: &'static str = "experiment";
    const RESOURCE_DESCRIPTION: &'static str =
        "This resource manages experiments associated with sample data collected during campaigns.";

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
            ("campaign_id", Self::ColumnType::CampaignId),
            ("experiment_date", Self::ColumnType::ExperimentDate),
            ("created_by", Self::ColumnType::CreatedBy),
            ("created_at", Self::ColumnType::CreatedAt),
            ("temperature_ramp", Self::ColumnType::TemperatureRamp),
            ("temperature_start", Self::ColumnType::TemperatureStart),
            ("temperature_end", Self::ColumnType::TemperatureEnd),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("name", Self::ColumnType::Name),
            ("campaign_id", Self::ColumnType::CampaignId),
            ("created_by", Self::ColumnType::CreatedBy),
            ("experiment_date", Self::ColumnType::ExperimentDate),
            ("created_at", Self::ColumnType::CreatedAt),
            ("temperature_ramp", Self::ColumnType::TemperatureRamp),
            ("temperature_start", Self::ColumnType::TemperatureStart),
            ("temperature_end", Self::ColumnType::TemperatureEnd),
        ]
    }
}
