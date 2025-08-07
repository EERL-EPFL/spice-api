use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "experiments")]
#[crudcrate(
    generate_router,
    api_struct = "Experiment",
    name_singular = "experiment",
    name_plural = "experiments",
    description = "Experiments track ice nucleation testing sessions with associated data and results."
)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[sea_orm(column_type = "Text", unique)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub name: String,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub username: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub performed_at: Option<DateTime<Utc>>,
    #[crudcrate(sortable, filterable)]
    pub temperature_ramp: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub temperature_start: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub temperature_end: Option<Decimal>,
    #[crudcrate(filterable)]
    pub is_calibration: bool,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub remarks: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub tray_configuration_id: Option<Uuid>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub last_updated: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::routes::trays::regions::models::Entity")]
    Regions,
    #[sea_orm(has_many = "crate::routes::assets::models::Entity")]
    S3Assets,
    #[sea_orm(has_many = "crate::routes::experiments::temperatures::models::Entity")]
    TemperatureReadings,
    #[sea_orm(
        belongs_to = "crate::routes::trays::configurations::models::Entity",
        from = "Column::TrayConfigurationId",
        to = "crate::routes::trays::configurations::models::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    TrayConfigurations,
    #[sea_orm(has_many = "crate::routes::experiments::phase_transitions::models::Entity")]
    WellPhaseTransitions,
}

impl Related<crate::routes::trays::regions::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Regions.def()
    }
}

impl Related<crate::routes::assets::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::S3Assets.def()
    }
}

impl Related<crate::routes::experiments::temperatures::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TemperatureReadings.def()
    }
}

impl Related<crate::routes::trays::configurations::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TrayConfigurations.def()
    }
}

impl Related<crate::routes::experiments::phase_transitions::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WellPhaseTransitions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
