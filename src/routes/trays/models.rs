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

#[derive(ToSchema, Serialize, Deserialize, Clone)]
struct Tray {
    name: Option<String>,
    qty_x_axis: Option<i32>,
    qty_y_axis: Option<i32>,
    well_relative_diameter: Option<Decimal>,
}
#[derive(ToSchema, Serialize, Deserialize, Clone)]
struct TrayAssignment {
    order_sequence: i16,
    rotation_degrees: i16,
    trays: Vec<Tray>,
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
                .map(|tray| Tray {
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
        let mut model: Self = model.into();
        // Sort tray assignments by order_sequence
        let mut tray_assignments = tray_assignments;
        tray_assignments.sort_by_key(|a| a.order_sequence);
        model.trays = tray_assignments;

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
        // If experiment_default is true, set all others to false
        if let Some(Some(experiment_default)) = update_data.experiment_default {
            if experiment_default {
                spice_entity::tray_configurations::Entity::update_many()
                    .col_expr(
                        spice_entity::tray_configurations::Column::ExperimentDefault,
                        Expr::value(false),
                    )
                    .filter(spice_entity::tray_configurations::Column::Id.ne(id))
                    .exec(db)
                    .await?;
            }
        }

        // Update the main tray configuration
        let existing: Self::ActiveModelType = Self::EntityType::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_PLURAL
            )))?
            .into();
        let updated_model = update_data.clone().merge_into_activemodel(existing);
        let _ = updated_model.update(db).await?;

        // Remove old assignments for this configuration
        let _ = spice_entity::tray_configuration_assignments::Entity::delete_many()
            .filter(
                spice_entity::tray_configuration_assignments::Column::TrayConfigurationId.eq(id),
            )
            .exec(db)
            .await?;

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
                let tray_model = tray_active.insert(db).await?;

                let assignment_active = spice_entity::tray_configuration_assignments::ActiveModel {
                    tray_id: Set(tray_model.id),
                    tray_configuration_id: Set(id),
                    order_sequence: Set(assignment.order_sequence),
                    rotation_degrees: Set(assignment.rotation_degrees),
                    created_at: Set(chrono::Utc::now().into()),
                    last_updated: Set(chrono::Utc::now().into()),
                };
                let _ = assignment_active.insert(db).await?;
            }
        }
        Self::get_one(db, id).await
    }

    async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<Uuid, DbErr> {
        // Delete all assignments for this configuration
        spice_entity::tray_configuration_assignments::Entity::delete_many()
            .filter(
                spice_entity::tray_configuration_assignments::Column::TrayConfigurationId.eq(id),
            )
            .exec(db)
            .await?;
        // Delete the configuration itself
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
        // Delete all assignments for these configurations
        spice_entity::tray_configuration_assignments::Entity::delete_many()
            .filter(
                spice_entity::tray_configuration_assignments::Column::TrayConfigurationId
                    .is_in(ids.clone()),
            )
            .exec(db)
            .await?;
        // Delete the configurations themselves
        let _ = <Self::EntityType as EntityTrait>::delete_many()
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
