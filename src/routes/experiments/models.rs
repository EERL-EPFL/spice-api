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
    experiment_code: String,
    campaign_id: Option<Uuid>,
    user_identifier: Option<String>,
    experiment_date: Option<DateTime<Utc>>,
    created_at: Option<DateTime<Utc>>,
    image_capture_started_at: Option<DateTime<Utc>>,
    image_capture_ended_at: Option<DateTime<Utc>>,
    temperature_ramp: Option<f64>,
    temperature_start: Option<f64>,
    temperature_end: Option<f64>,
    cooling_rate: Option<f64>,
    temperature_calibration_slope: Option<f64>,
    temperature_calibration_intercept: Option<f64>,
}

impl From<Model> for Experiment {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            experiment_code: model.experiment_code,
            campaign_id: model.campaign_id,
            user_identifier: model.user_identifier,
            experiment_date: model.experiment_date,
            created_at: model.created_at,
            image_capture_started_at: model.image_capture_started_at,
            image_capture_ended_at: model.image_capture_ended_at,
            temperature_ramp: model.temperature_ramp,
            temperature_start: model.temperature_start,
            temperature_end: model.temperature_end,
            cooling_rate: model.cooling_rate,
            temperature_calibration_slope: model.temperature_calibration_slope,
            temperature_calibration_intercept: model.temperature_calibration_intercept,
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
            ("experiment_code", Self::ColumnType::ExperimentCode),
            ("campaign_id", Self::ColumnType::CampaignId),
            ("user_identifier", Self::ColumnType::UserIdentifier),
            ("experiment_date", Self::ColumnType::ExperimentDate),
            ("created_at", Self::ColumnType::CreatedAt),
            (
                "image_capture_started_at",
                Self::ColumnType::ImageCaptureStartedAt,
            ),
            (
                "image_capture_ended_at",
                Self::ColumnType::ImageCaptureEndedAt,
            ),
            ("temperature_ramp", Self::ColumnType::TemperatureRamp),
            ("temperature_start", Self::ColumnType::TemperatureStart),
            ("temperature_end", Self::ColumnType::TemperatureEnd),
            ("cooling_rate", Self::ColumnType::CoolingRate),
            (
                "temperature_calibration_slope",
                Self::ColumnType::TemperatureCalibrationSlope,
            ),
            (
                "temperature_calibration_intercept",
                Self::ColumnType::TemperatureCalibrationIntercept,
            ),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("name", Self::ColumnType::ExperimentCode),
            ("campaign_id", Self::ColumnType::CampaignId),
            ("user_identifier", Self::ColumnType::UserIdentifier),
            ("experiment_date", Self::ColumnType::ExperimentDate),
            ("created_at", Self::ColumnType::CreatedAt),
            (
                "image_capture_started_at",
                Self::ColumnType::ImageCaptureStartedAt,
            ),
            (
                "image_capture_ended_at",
                Self::ColumnType::ImageCaptureEndedAt,
            ),
            ("temperature_ramp", Self::ColumnType::TemperatureRamp),
            ("temperature_start", Self::ColumnType::TemperatureStart),
            ("temperature_end", Self::ColumnType::TemperatureEnd),
            ("cooling_rate", Self::ColumnType::CoolingRate),
        ]
    }
}
