use crate::external::s3::delete_from_s3;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel, traits::MergeIntoActiveModel};
use sea_orm::{
    ActiveValue, Condition, DatabaseConnection, EntityTrait, Order, QueryOrder, QuerySelect,
    entity::prelude::*,
};
use serde::{Deserialize, Serialize};
use spice_entity::s3_assets::Model as S3Assets;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "spice_entity::s3_assets::ActiveModel"]
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

impl From<S3Assets> for Asset {
    fn from(model: S3Assets) -> Self {
        Self {
            id: model.id,
            experiment_id: model.experiment_id,
            original_filename: model.original_filename,
            s3_key: model.s3_key,
            size_bytes: model.size_bytes,
            uploaded_by: model.uploaded_by,
            uploaded_at: model.uploaded_at.into(),
            is_deleted: model.is_deleted,
            r#type: model.r#type,
            role: model.role,
            last_updated: model.last_updated.into(),
            created_at: model.created_at.into(),
        }
    }
}

#[async_trait]
impl CRUDResource for Asset {
    type EntityType = spice_entity::s3_assets::Entity;
    type ColumnType = spice_entity::s3_assets::Column;
    type ActiveModelType = spice_entity::s3_assets::ActiveModel;
    type CreateModel = AssetCreate;
    type UpdateModel = AssetUpdate;

    const ID_COLUMN: Self::ColumnType = spice_entity::s3_assets::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "assets";
    const RESOURCE_NAME_SINGULAR: &'static str = "asset";
    const RESOURCE_DESCRIPTION: &'static str = "This resource represents assets stored in S3, including metadata such as file size, type, and upload details.";

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
        Ok(models.into_iter().map(Self::from).collect())
    }

    async fn get_one(db: &DatabaseConnection, id: Uuid) -> Result<Self, DbErr> {
        let model: S3Assets = match Self::EntityType::find_by_id(id).one(db).await? {
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
    ) -> Result<Self, DbErr> {
        let existing: Self::ActiveModelType = Self::EntityType::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_PLURAL
            )))?
            .into();

        let updated_model = update_data.merge_into_activemodel(existing)?;
        let updated = updated_model.update(db).await?;
        Ok(Self::from(updated))
    }

    async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<Uuid, DbErr> {
        // Fetch the asset to get its S3 key
        let asset =
            Self::EntityType::find_by_id(id)
                .one(db)
                .await?
                .ok_or(DbErr::RecordNotFound(format!(
                    "{} not found",
                    Self::RESOURCE_NAME_SINGULAR
                )))?;

        // Delete the asset from S3 (replace this with actual S3 deletion logic)
        if let Err(e) = delete_from_s3(&asset.s3_key).await {
            return Err(DbErr::Custom(format!(
                "Failed to delete S3 asset with key {}: {}",
                asset.s3_key, e
            )));
        }

        // Proceed with deleting the database record
        let res = <Self::EntityType as EntityTrait>::delete_by_id(id)
            .exec(db)
            .await?;
        match res.rows_affected {
            0 => Err(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_SINGULAR
            ))),
            _ => Ok(id),
        }
    }

    async fn delete_many(db: &DatabaseConnection, ids: Vec<Uuid>) -> Result<Vec<Uuid>, DbErr> {
        // Fetch the assets to get their S3 keys
        let assets = Self::EntityType::find()
            .filter(Self::ID_COLUMN.is_in(ids.clone()))
            .all(db)
            .await?;

        // Delete the assets from S3 (replace this with actual S3 deletion logic)
        for asset in &assets {
            if let Err(e) = delete_from_s3(&asset.s3_key).await {
                return Err(DbErr::Custom(format!(
                    "Failed to delete S3 asset with key {}: {}",
                    asset.s3_key, e
                )));
            }
        }

        // Proceed with deleting the database records
        Self::EntityType::delete_many()
            .filter(Self::ID_COLUMN.is_in(ids.clone()))
            .exec(db)
            .await?;
        Ok(ids)
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
