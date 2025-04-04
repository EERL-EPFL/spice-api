use super::db::Model;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel};
use sea_orm::{
    ActiveValue, Condition, DatabaseConnection, EntityTrait, Order, QueryOrder, QuerySelect,
    entity::prelude::*,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "super::db::ActiveModel"]
pub struct Asset {
    #[crudcrate(update_model = false, create_model = false, on_create = Uuid::new_v4())]
    id: Uuid,
    experiment_id: Option<Uuid>,
    original_filename: String,
    s3_key: String,
    size_bytes: Option<i64>,
    uploaded_by: Option<String>,
    uploaded_at: DateTime<Utc>,
    is_deleted: bool,
    r#type: String,
    role: Option<String>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    last_updated: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    created_at: DateTime<Utc>,
}

impl From<Model> for Asset {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            experiment_id: model.experiment_id,
            original_filename: model.original_filename,
            s3_key: model.s3_key,
            size_bytes: model.size_bytes,
            uploaded_by: model.uploaded_by,
            uploaded_at: model.uploaded_at,
            is_deleted: model.is_deleted,
            r#type: model.r#type,
            role: model.role,
            last_updated: model.last_updated,
            created_at: model.created_at,
        }
    }
}

#[async_trait]
impl CRUDResource for Asset {
    type EntityType = super::db::Entity;
    type ColumnType = super::db::Column;
    type ModelType = super::db::Model;
    type ActiveModelType = super::db::ActiveModel;
    type ApiModel = Asset;
    type CreateModel = AssetCreate;
    type UpdateModel = AssetUpdate;

    const ID_COLUMN: Self::ColumnType = super::db::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "assets";
    const RESOURCE_NAME_SINGULAR: &'static str = "asset";
    const RESOURCE_DESCRIPTION: &'static str = "This resource represents assets stored in S3, including metadata such as file size, type, and upload details.";

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
        let model: Model = match Self::EntityType::find_by_id(id).one(db).await? {
            Some(model) => model,
            None => {
                return Err(DbErr::RecordNotFound(format!(
                    "{} not found",
                    Self::RESOURCE_NAME_SINGULAR
                )));
            }
        };

        Ok(model.into())
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
            ("experiment_id", Self::ColumnType::ExperimentId),
            ("original_filename", Self::ColumnType::OriginalFilename),
            ("s3_key", Self::ColumnType::S3Key),
            ("size_bytes", Self::ColumnType::SizeBytes),
            ("uploaded_by", Self::ColumnType::UploadedBy),
            ("uploaded_at", Self::ColumnType::UploadedAt),
            ("last_updated", Self::ColumnType::LastUpdated),
            ("created_at", Self::ColumnType::CreatedAt),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("experiment_id", Self::ColumnType::ExperimentId),
            ("original_filename", Self::ColumnType::OriginalFilename),
            ("s3_key", Self::ColumnType::S3Key),
            ("uploaded_by", Self::ColumnType::UploadedBy),
            ("is_deleted", Self::ColumnType::IsDeleted),
            ("r#type", Self::ColumnType::Type),
            ("role", Self::ColumnType::Role),
        ]
    }
}
