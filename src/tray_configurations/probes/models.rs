use chrono::{DateTime, Utc};
use crudcrate::EntityToModels;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "probes")]
#[crudcrate(api_struct = "Probe")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[crudcrate(sortable, filterable, list_model = false, create_model = false)]
    pub tray_id: Uuid,
    #[crudcrate(sortable, filterable, fulltext)]
    pub name: String,
    #[crudcrate(sortable, filterable)]
    pub data_column_index: i32,
    #[crudcrate(sortable, filterable)]
    pub position_x: Decimal,
    #[crudcrate(sortable, filterable)]
    pub position_y: Decimal,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub last_updated: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::tray_configurations::trays::models::Entity",
        from = "Column::TrayId",
        to = "crate::tray_configurations::trays::models::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Trays,
    #[sea_orm(has_many = "crate::experiments::probe_temperature_readings::models::Entity")]
    ProbeTemperatureReadings,
}

impl Related<crate::tray_configurations::trays::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Trays.def()
    }
}

impl Related<crate::experiments::probe_temperature_readings::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProbeTemperatureReadings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}