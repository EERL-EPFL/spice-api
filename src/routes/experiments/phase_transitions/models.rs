use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "well_phase_transitions")]
#[crudcrate(
    generate_router,
    api_struct = "WellPhaseTransition",
    name_singular = "well_phase_transition",
    name_plural = "well_phase_transitions",
    description = "Tracks phase state transitions (liquid to frozen) for individual wells during experiments."
)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub well_id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub experiment_id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub temperature_reading_id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub timestamp: DateTime<Utc>,
    #[crudcrate(sortable, filterable)]
    pub previous_state: i32,
    #[crudcrate(sortable, filterable)]
    pub new_state: i32,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::routes::experiments::models::Entity",
        from = "Column::ExperimentId",
        to = "crate::routes::experiments::models::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Experiments,
    #[sea_orm(
        belongs_to = "crate::routes::experiments::temperatures::models::Entity",
        from = "Column::TemperatureReadingId",
        to = "crate::routes::experiments::temperatures::models::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    TemperatureReadings,
    #[sea_orm(
        belongs_to = "crate::routes::tray_configurations::wells::models::Entity",
        from = "Column::WellId",
        to = "crate::routes::tray_configurations::wells::models::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Wells,
}

impl Related<crate::routes::experiments::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Experiments.def()
    }
}

impl Related<crate::routes::experiments::temperatures::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TemperatureReadings.def()
    }
}

impl Related<crate::routes::tray_configurations::wells::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Wells.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
