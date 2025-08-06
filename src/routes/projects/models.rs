use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel, traits::MergeIntoActiveModel};
use sea_orm::{
    ActiveValue, Condition, DatabaseConnection, EntityTrait, Order, QueryOrder, QuerySelect,
    entity::prelude::*,
};
use serde::{Deserialize, Serialize};
use spice_entity::projects::Model;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "spice_entity::projects::ActiveModel"]
pub struct Project {
    #[crudcrate(update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    pub last_updated: DateTime<Utc>,
    pub name: String,
    pub note: Option<String>,
    pub colour: Option<String>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    pub locations: Vec<crate::routes::campaigns::models::Location>,
}

impl From<Model> for Project {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            created_at: model.created_at.into(),
            last_updated: model.last_updated.into(),
            name: model.name,
            note: model.note,
            colour: model.colour,
            locations: vec![],
        }
    }
}

#[async_trait]
impl CRUDResource for Project {
    type EntityType = spice_entity::projects::Entity;
    type ColumnType = spice_entity::projects::Column;
    type ActiveModelType = spice_entity::projects::ActiveModel;
    type CreateModel = ProjectCreate;
    type UpdateModel = ProjectUpdate;

    const ID_COLUMN: Self::ColumnType = spice_entity::projects::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "projects";
    const RESOURCE_NAME_SINGULAR: &'static str = "project";
    const RESOURCE_DESCRIPTION: &'static str = "Projects provide a way to organize locations hierarchically. Each project can contain multiple locations and provides visual organization through color coding.";

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
        let model: Model = match Self::EntityType::find_by_id(id).one(db).await? {
            Some(model) => model,
            None => {
                return Err(DbErr::RecordNotFound(format!(
                    "{} not found",
                    Self::RESOURCE_NAME_SINGULAR
                )));
            }
        };

        let locations = model
            .find_related(spice_entity::locations::Entity)
            .all(db)
            .await?;

        let location_objs: Vec<crate::routes::campaigns::models::Location> = locations
            .into_iter()
            .map(std::convert::Into::into)
            .collect();

        let mut project: Self = model.into();
        project.locations = location_objs;

        Ok(project)
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

    fn sortable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("id", Self::ColumnType::Id),
            ("name", Self::ColumnType::Name),
            ("note", Self::ColumnType::Note),
            ("colour", Self::ColumnType::Colour),
            ("created_at", Self::ColumnType::CreatedAt),
            ("last_updated", Self::ColumnType::LastUpdated),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("name", Self::ColumnType::Name),
            ("note", Self::ColumnType::Note),
            ("colour", Self::ColumnType::Colour),
        ]
    }
}
