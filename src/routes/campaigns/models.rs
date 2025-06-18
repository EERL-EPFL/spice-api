use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel, traits::MergeIntoActiveModel};
use sea_orm::{
    ActiveValue, Condition, DatabaseConnection, EntityTrait, Order, QueryOrder, QuerySelect,
    entity::prelude::*,
};
use serde::{Deserialize, Serialize};
use spice_entity::locations::Model;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "spice_entity::locations::ActiveModel"]
pub struct Location {
    #[crudcrate(update_model = false, update_model = false, on_create = Uuid::new_v4())]
    id: Uuid,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    last_updated: DateTime<Utc>,
    comment: Option<String>,
    name: String,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    project_id: Option<Uuid>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    experiments: Vec<crate::routes::experiments::models::Experiment>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    samples: Vec<crate::routes::samples::models::Sample>,
}

impl From<Model> for Location {
    fn from(model: Model) -> Self {
        Self {
            last_updated: model.last_updated.into(),
            created_at: model.created_at.into(),
            comment: model.comment,
            id: model.id,
            name: model.name,
            start_date: model.start_date.map(|dt| dt.with_timezone(&Utc)),
            end_date: model.end_date.map(|dt| dt.with_timezone(&Utc)),
            project_id: model.project_id,
            experiments: vec![],
            samples: vec![],
        }
    }
}

#[async_trait]
impl CRUDResource for Location {
    type EntityType = spice_entity::locations::Entity;
    type ColumnType = spice_entity::locations::Column;
    type ActiveModelType = spice_entity::locations::ActiveModel;
    type CreateModel = LocationCreate;
    type UpdateModel = LocationUpdate;

    const ID_COLUMN: Self::ColumnType = spice_entity::locations::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "locations";
    const RESOURCE_NAME_SINGULAR: &'static str = "location";
    const RESOURCE_DESCRIPTION: &'static str = "This resource allows the data hierarchically beneath each area to be allocated to a specific location. This is useful for grouping data together for analysis. Locations can be assigned to projects for better organization.";

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

        let samples = model
            .find_related(spice_entity::samples::Entity)
            .all(db)
            .await?;

        let mut sample_objs: Vec<crate::routes::samples::models::Sample> = vec![];
        for sample in samples {
            let treatments = sample
                .find_related(spice_entity::treatments::Entity)
                .all(db)
                .await?;

            let mut sample_obj: crate::routes::samples::models::Sample = sample.into();
            sample_obj.treatments = treatments
                .into_iter()
                .map(std::convert::Into::into)
                .collect();
            sample_objs.push(sample_obj);
        }

        // Get the experiments related to each location, by finding all of the
        // regions that the sample treatments
        // have been used in, and then the experiment via the region
        // Therefore samples -> treatments -> regions -> experiments

        // So first, get a list of all treatment IDs from the samples above
        let treatment_ids: Vec<Uuid> = sample_objs
            .iter()
            .flat_map(|s| s.treatments.iter().map(|t| t.id))
            .collect();
        println!("Treatment IDs: {:?}", treatment_ids);
        // Then find all regions that have these treatments
        let regions = spice_entity::regions::Entity::find()
            .filter(spice_entity::regions::Column::TreatmentId.is_in(treatment_ids))
            .all(db)
            .await?;
        println!("Regions found: {:?}", regions.len());

        // Now find all experiments related to these regions (experiment id is in region)
        let experiments = spice_entity::experiments::Entity::find()
            .filter(
                spice_entity::experiments::Column::Id
                    .is_in(regions.iter().map(|r| r.experiment_id).collect::<Vec<_>>()),
            )
            .all(db)
            .await?;

        // Convert experiments to the appropriate model
        let experiments: Vec<crate::routes::experiments::models::Experiment> = experiments
            .into_iter()
            .map(std::convert::Into::into)
            .collect();

        let mut model: Self = model.into();
        model.experiments = experiments; // Assign experiments to the model
        model.samples = sample_objs;

        Ok(model)
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

        let updated_model = update_data.merge_into_activemodel(existing);
        let updated = updated_model.update(db).await?;
        Ok(Self::from(updated))
    }

    fn sortable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("id", Self::ColumnType::Id),
            ("name", Self::ColumnType::Name),
            ("comment", Self::ColumnType::Comment),
            ("last_updated", Self::ColumnType::LastUpdated),
            ("start_date", Self::ColumnType::StartDate),
            ("end_date", Self::ColumnType::EndDate),
            ("project_id", Self::ColumnType::ProjectId),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("name", Self::ColumnType::Name),
            ("comment", Self::ColumnType::Comment),
            ("project_id", Self::ColumnType::ProjectId),
        ]
    }
}
