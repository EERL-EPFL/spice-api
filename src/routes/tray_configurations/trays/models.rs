use chrono::{DateTime, Utc};
use crudcrate::EntityToModels;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

// This will become the new 'trays' table after migration
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "trays")]
#[crudcrate(api_struct = "Tray")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub tray_configuration_id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub order_sequence: i32,
    #[crudcrate(sortable, filterable)]
    pub rotation_degrees: i32,
    #[crudcrate(sortable, filterable, fulltext)]
    pub name: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub qty_x_axis: Option<i32>,
    #[crudcrate(sortable, filterable)]
    pub qty_y_axis: Option<i32>,
    #[crudcrate(sortable, filterable)]
    pub well_relative_diameter: Option<Decimal>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub last_updated: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::routes::tray_configurations::models::Entity",
        from = "Column::TrayConfigurationId",
        to = "crate::routes::tray_configurations::models::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    TrayConfigurations,
    #[sea_orm(has_many = "crate::routes::tray_configurations::wells::models::Entity")]
    Wells,
}

impl Related<crate::routes::tray_configurations::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TrayConfigurations.def()
    }
}

impl Related<crate::routes::tray_configurations::wells::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Wells.def()
    }
}

// Input model for creating trays (without id field)
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TrayCreateInput {
    pub order_sequence: i32,
    pub rotation_degrees: i32,
    pub name: Option<String>,
    pub qty_x_axis: Option<i32>,
    pub qty_y_axis: Option<i32>,
    pub well_relative_diameter: Option<Decimal>,
}

impl ActiveModelBehavior for ActiveModel {}
