use crate::routes::experiments::models::Experiment;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel, traits::MergeIntoActiveModel};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, Set, entity::prelude::*, prelude::Expr,
};
use serde::{Deserialize, Serialize};
use spice_entity::tray_configurations::Model;
use utoipa::ToSchema;
use uuid::Uuid;

// Individual Tray model for the /api/trays endpoint
#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "spice_entity::trays::ActiveModel"]
pub struct Tray {
    #[crudcrate(update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    pub name: Option<String>,
    pub qty_x_axis: Option<i32>,
    pub qty_y_axis: Option<i32>,
    pub well_relative_diameter: Option<Decimal>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    pub last_updated: DateTime<Utc>,
}

impl From<spice_entity::trays::Model> for Tray {
    fn from(model: spice_entity::trays::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            qty_x_axis: model.qty_x_axis,
            qty_y_axis: model.qty_y_axis,
            well_relative_diameter: model.well_relative_diameter,
            created_at: model.created_at.into(),
            last_updated: model.last_updated.into(),
        }
    }
}

#[async_trait]
impl CRUDResource for Tray {
    type EntityType = spice_entity::trays::Entity;
    type ColumnType = spice_entity::trays::Column;
    type ActiveModelType = spice_entity::trays::ActiveModel;
    type CreateModel = TrayCreate;
    type UpdateModel = TrayUpdate;

    const ID_COLUMN: Self::ColumnType = spice_entity::trays::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "trays";
    const RESOURCE_NAME_SINGULAR: &'static str = "tray";
    const RESOURCE_DESCRIPTION: &'static str =
        "This endpoint manages individual trays used in experiments.";

    async fn get_one(db: &DatabaseConnection, id: Uuid) -> Result<Self, DbErr> {
        let model =
            Self::EntityType::find_by_id(id)
                .one(db)
                .await?
                .ok_or(DbErr::RecordNotFound(format!(
                    "{} not found",
                    Self::RESOURCE_NAME_SINGULAR
                )))?;
        Ok(model.into())
    }

    // async fn create(
    //     db: &DatabaseConnection,
    //     create_data: Self::CreateModel,
    // ) -> Result<Self, DbErr> {
    //     let active_model = create_data.into_active_model();
    //     let model = active_model.insert(db).await?;
    //     Ok(model.into())
    // }

    async fn update(
        db: &DatabaseConnection,
        id: Uuid,
        update_data: Self::UpdateModel,
    ) -> Result<Self, DbErr> {
        let existing = Self::EntityType::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_SINGULAR
            )))?
            .into();
        let updated_model = update_data.merge_into_activemodel(existing);
        let model = updated_model.update(db).await?;
        Ok(model.into())
    }

    async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<Uuid, DbErr> {
        let res = Self::EntityType::delete_by_id(id).exec(db).await?;
        if res.rows_affected == 0 {
            Err(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_SINGULAR
            )))
        } else {
            Ok(id)
        }
    }

    async fn delete_many(db: &DatabaseConnection, ids: Vec<Uuid>) -> Result<Vec<Uuid>, DbErr> {
        let _ = Self::EntityType::delete_many()
            .filter(Self::ID_COLUMN.is_in(ids.clone()))
            .exec(db)
            .await?;
        Ok(ids)
    }

    fn sortable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("id", Self::ColumnType::Id),
            ("name", Self::ColumnType::Name),
            ("last_updated", Self::ColumnType::LastUpdated),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![("name", Self::ColumnType::Name)]
    }
}

// Inner tray struct used within TrayConfiguration
#[derive(ToSchema, Serialize, Deserialize, Clone)]
struct TrayInfo {
    name: Option<String>,
    qty_x_axis: Option<i32>,
    qty_y_axis: Option<i32>,
    well_relative_diameter: Option<Decimal>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
struct TrayAssignment {
    order_sequence: i16,
    rotation_degrees: i16,
    trays: Vec<TrayInfo>,
}

impl TrayAssignment {
    // A function that accepts db and a tray configuration id then populates
    // the tray assignment with an array of Tray structs that are related to the tray configuration
    async fn from_tray_configuration(
        db: &DatabaseConnection,
        tray_configuration_id: Uuid,
    ) -> anyhow::Result<Vec<Self>> {
        let assignments = spice_entity::tray_configuration_assignments::Entity::find()
            .filter(
                spice_entity::tray_configuration_assignments::Column::TrayConfigurationId
                    .eq(tray_configuration_id),
            )
            .all(db)
            .await?;

        let mut tray_assignments = Vec::new();
        for assignment in assignments {
            let trays = spice_entity::trays::Entity::find()
                .filter(spice_entity::trays::Column::Id.eq(assignment.tray_id))
                .all(db)
                .await?
                .into_iter()
                .map(|tray| TrayInfo {
                    name: tray.name,
                    qty_x_axis: tray.qty_x_axis,
                    qty_y_axis: tray.qty_y_axis,
                    well_relative_diameter: tray.well_relative_diameter,
                })
                .collect();

            tray_assignments.push(Self {
                order_sequence: assignment.order_sequence,
                rotation_degrees: assignment.rotation_degrees,
                trays,
            });
        }
        Ok(tray_assignments)
    }
}

#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "spice_entity::tray_configurations::ActiveModel"]
pub struct TrayConfiguration {
    #[crudcrate(update_model = false, update_model = false, on_create = Uuid::new_v4())]
    id: Uuid,
    name: Option<String>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    last_updated: DateTime<Utc>,
    experiment_default: bool,
    #[crudcrate(non_db_attr = true, default = vec![])]
    trays: Vec<TrayAssignment>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    associated_experiments: Vec<Experiment>,
}

impl From<Model> for TrayConfiguration {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            last_updated: model.last_updated.into(),
            created_at: model.created_at.into(),
            experiment_default: model.experiment_default,
            trays: vec![],
            associated_experiments: vec![],
        }
    }
}

#[async_trait]
impl CRUDResource for TrayConfiguration {
    type EntityType = spice_entity::tray_configurations::Entity;
    type ColumnType = spice_entity::tray_configurations::Column;
    type ActiveModelType = spice_entity::tray_configurations::ActiveModel;
    type CreateModel = TrayConfigurationCreate;
    type UpdateModel = TrayConfigurationUpdate;

    const ID_COLUMN: Self::ColumnType = spice_entity::tray_configurations::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "tray_configurations";
    const RESOURCE_NAME_SINGULAR: &'static str = "tray_configuration";
    const RESOURCE_DESCRIPTION: &'static str = "This endpoint manages tray configurations, which define the setup of trays used in experiments.";

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

        let tray_assignments = TrayAssignment::from_tray_configuration(db, model.id).await;
        let tray_assignments = match tray_assignments {
            Ok(assignments) => assignments,
            Err(e) => {
                return Err(DbErr::Custom(format!(
                    "Failed to fetch tray assignments: {e}"
                )));
            }
        };

        let experiments: Vec<Experiment> = match spice_entity::experiments::Entity::find()
            .filter(spice_entity::experiments::Column::TrayConfigurationId.eq(id))
            .all(db)
            .await
        {
            Ok(experiments) => experiments.into_iter().map(Into::into).collect(),
            Err(e) => {
                return Err(DbErr::Custom(format!(
                    "Failed to fetch associated experiments: {e}"
                )));
            }
        };

        let mut model: Self = model.into();
        // Sort tray assignments by order_sequence
        let mut tray_assignments = tray_assignments;
        tray_assignments.sort_by_key(|a| a.order_sequence);
        model.trays = tray_assignments;
        model.associated_experiments = experiments;

        Ok(model)
    }

    async fn create(
        db: &DatabaseConnection,
        create_data: Self::CreateModel,
    ) -> Result<Self, DbErr> {
        // If experiment_default is true, set all others to false
        if create_data.experiment_default {
            spice_entity::tray_configurations::Entity::update_many()
                .col_expr(
                    spice_entity::tray_configurations::Column::ExperimentDefault,
                    Expr::value(false),
                )
                .exec(db)
                .await?;
        }

        // Create a new UUID for this configuration
        let tray_config_id = Uuid::new_v4();

        // Insert the main tray configuration with explicit ID
        let active_model = spice_entity::tray_configurations::ActiveModel {
            id: Set(tray_config_id),
            name: Set(create_data.name.clone()),
            experiment_default: Set(create_data.experiment_default),
            created_at: Set(chrono::Utc::now().into()),
            last_updated: Set(chrono::Utc::now().into()),
        };
        let _inserted = active_model.insert(db).await?;

        // Insert tray assignments and trays
        for assignment in create_data.trays {
            for tray in assignment.trays {
                // Create tray first
                let tray_active = spice_entity::trays::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    name: Set(tray.name.clone()),
                    qty_x_axis: Set(tray.qty_x_axis),
                    qty_y_axis: Set(tray.qty_y_axis),
                    well_relative_diameter: Set(tray.well_relative_diameter),
                    created_at: Set(chrono::Utc::now().into()),
                    last_updated: Set(chrono::Utc::now().into()),
                };
                let tray_model = tray_active.insert(db).await?;

                // Insert assignment with the explicitly created tray_config_id
                let assignment_active = spice_entity::tray_configuration_assignments::ActiveModel {
                    tray_id: Set(tray_model.id),
                    tray_configuration_id: Set(tray_config_id),
                    order_sequence: Set(assignment.order_sequence),
                    rotation_degrees: Set(assignment.rotation_degrees),
                    created_at: Set(chrono::Utc::now().into()),
                    last_updated: Set(chrono::Utc::now().into()),
                };
                let _ = assignment_active.insert(db).await?;
            }
        }
        // Return the full model with assignments
        Self::get_one(db, tray_config_id).await
    }

    async fn update(
        db: &DatabaseConnection,
        id: Uuid,
        update_data: Self::UpdateModel,
    ) -> Result<Self, DbErr> {
        use sea_orm::TransactionTrait;

        // Use a transaction to ensure atomicity
        let txn = db.begin().await?;

        // If experiment_default is true, set all others to false
        if let Some(Some(experiment_default)) = update_data.experiment_default {
            if experiment_default {
                spice_entity::tray_configurations::Entity::update_many()
                    .col_expr(
                        spice_entity::tray_configurations::Column::ExperimentDefault,
                        Expr::value(false),
                    )
                    .filter(spice_entity::tray_configurations::Column::Id.ne(id))
                    .exec(&txn)
                    .await?;
            }
        }

        // Update the main tray configuration
        let existing: Self::ActiveModelType = Self::EntityType::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_PLURAL
            )))?
            .into();
        let updated_model = update_data.clone().merge_into_activemodel(existing);
        let _ = updated_model.update(&txn).await?;

        // First, get all existing tray IDs for this configuration to clean them up later
        let existing_assignments = spice_entity::tray_configuration_assignments::Entity::find()
            .filter(
                spice_entity::tray_configuration_assignments::Column::TrayConfigurationId.eq(id),
            )
            .all(&txn)
            .await?;

        let existing_tray_ids: Vec<Uuid> = existing_assignments.iter().map(|a| a.tray_id).collect();

        // Remove old assignments for this configuration
        let _ = spice_entity::tray_configuration_assignments::Entity::delete_many()
            .filter(
                spice_entity::tray_configuration_assignments::Column::TrayConfigurationId.eq(id),
            )
            .exec(&txn)
            .await?;

        // Now remove the orphaned trays
        if !existing_tray_ids.is_empty() {
            let _ = spice_entity::trays::Entity::delete_many()
                .filter(spice_entity::trays::Column::Id.is_in(existing_tray_ids))
                .exec(&txn)
                .await?;
        }

        // Insert new assignments and trays
        for assignment in update_data.trays {
            for tray in assignment.trays {
                let tray_active = spice_entity::trays::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    name: Set(tray.name.clone()),
                    qty_x_axis: Set(tray.qty_x_axis),
                    qty_y_axis: Set(tray.qty_y_axis),
                    well_relative_diameter: Set(tray.well_relative_diameter),
                    created_at: Set(chrono::Utc::now().into()),
                    last_updated: Set(chrono::Utc::now().into()),
                };
                let tray_model = tray_active.insert(&txn).await?;

                let assignment_active = spice_entity::tray_configuration_assignments::ActiveModel {
                    tray_id: Set(tray_model.id),
                    tray_configuration_id: Set(id),
                    order_sequence: Set(assignment.order_sequence),
                    rotation_degrees: Set(assignment.rotation_degrees),
                    created_at: Set(chrono::Utc::now().into()),
                    last_updated: Set(chrono::Utc::now().into()),
                };
                let _ = assignment_active.insert(&txn).await?;
            }
        }

        // Commit the transaction
        txn.commit().await?;

        Self::get_one(db, id).await
    }

    async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<Uuid, DbErr> {
        use sea_orm::TransactionTrait;

        // Use a transaction for proper cleanup
        let txn = db.begin().await?;

        // First, get all tray IDs for this configuration
        let assignments = spice_entity::tray_configuration_assignments::Entity::find()
            .filter(
                spice_entity::tray_configuration_assignments::Column::TrayConfigurationId.eq(id),
            )
            .all(&txn)
            .await?;

        let tray_ids: Vec<Uuid> = assignments.iter().map(|a| a.tray_id).collect();

        // Delete all assignments for this configuration
        spice_entity::tray_configuration_assignments::Entity::delete_many()
            .filter(
                spice_entity::tray_configuration_assignments::Column::TrayConfigurationId.eq(id),
            )
            .exec(&txn)
            .await?;

        // Delete the associated trays
        if !tray_ids.is_empty() {
            spice_entity::trays::Entity::delete_many()
                .filter(spice_entity::trays::Column::Id.is_in(tray_ids))
                .exec(&txn)
                .await?;
        }

        // Delete the configuration itself
        let res = <Self::EntityType as EntityTrait>::delete_by_id(id)
            .exec(&txn)
            .await?;

        if res.rows_affected == 0 {
            Err(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_SINGULAR
            )))
        } else {
            txn.commit().await?;
            Ok(id)
        }
    }

    async fn delete_many(db: &DatabaseConnection, ids: Vec<Uuid>) -> Result<Vec<Uuid>, DbErr> {
        use sea_orm::TransactionTrait;

        // Use a transaction for proper cleanup
        let txn = db.begin().await?;

        // First, get all tray IDs for these configurations
        let assignments = spice_entity::tray_configuration_assignments::Entity::find()
            .filter(
                spice_entity::tray_configuration_assignments::Column::TrayConfigurationId
                    .is_in(ids.clone()),
            )
            .all(&txn)
            .await?;

        let tray_ids: Vec<Uuid> = assignments.iter().map(|a| a.tray_id).collect();

        // Delete all assignments for these configurations
        spice_entity::tray_configuration_assignments::Entity::delete_many()
            .filter(
                spice_entity::tray_configuration_assignments::Column::TrayConfigurationId
                    .is_in(ids.clone()),
            )
            .exec(&txn)
            .await?;

        // Delete the associated trays
        if !tray_ids.is_empty() {
            spice_entity::trays::Entity::delete_many()
                .filter(spice_entity::trays::Column::Id.is_in(tray_ids))
                .exec(&txn)
                .await?;
        }

        // Delete the configurations themselves
        let _ = <Self::EntityType as EntityTrait>::delete_many()
            .filter(Self::ID_COLUMN.is_in(ids.clone()))
            .exec(&txn)
            .await?;

        txn.commit().await?;
        Ok(ids)
    }

    fn sortable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("id", Self::ColumnType::Id),
            ("name", Self::ColumnType::Name),
            ("last_updated", Self::ColumnType::LastUpdated),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![("name", Self::ColumnType::Name)]
    }
}
