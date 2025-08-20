use chrono::{DateTime, Utc};
use crudcrate::EntityToModels;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "probe_temperature_readings")]
#[crudcrate(api_struct = "ProbeTemperatureReading")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub probe_id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub temperature_reading_id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub temperature: Decimal,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::tray_configurations::probes::models::Entity",
        from = "Column::ProbeId",
        to = "crate::tray_configurations::probes::models::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Probes,
    #[sea_orm(
        belongs_to = "crate::experiments::temperatures::models::Entity",
        from = "Column::TemperatureReadingId",
        to = "crate::experiments::temperatures::models::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    TemperatureReadings,
}

impl Related<crate::tray_configurations::probes::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Probes.def()
    }
}

impl Related<crate::experiments::temperatures::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TemperatureReadings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}