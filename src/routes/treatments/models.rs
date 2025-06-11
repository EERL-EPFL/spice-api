use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel, traits::MergeIntoActiveModel};
use sea_orm::{ActiveModelTrait, Set};
use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait, entity::prelude::*};
use serde::{Deserialize, Serialize};
use spice_entity::treatments::Model;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "spice_entity::treatments::ActiveModel"]
pub struct Treatment {
    #[crudcrate(update_model = false, create_model = false, on_create = Uuid::new_v4())]
    id: Uuid,
    name: Option<String>,
    notes: Option<String>,
    sample_id: Option<Uuid>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    last_updated: DateTime<Utc>,
    enzyme_volume_microlitres: Option<f64>,
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
            enzyme_volume_microlitres: model.enzyme_volume_microlitres,
        }
    }
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
}
