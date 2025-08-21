use chrono::{DateTime, Utc};
use crudcrate::EntityToModels;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use uuid::Uuid;


#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "trays")]
#[crudcrate(api_struct = "Tray")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[crudcrate(sortable, filterable, list_model = false, create_model = false)]
    pub tray_configuration_id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub order_sequence: i32,
    #[crudcrate(sortable, filterable)]
    pub rotation_degrees: i32,
    #[crudcrate(sortable, filterable, fulltext)]
    pub name: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub qty_cols: Option<i32>,
    #[crudcrate(sortable, filterable)]
    pub qty_rows: Option<i32>,
    #[crudcrate(sortable, filterable)]
    pub well_relative_diameter: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub upper_left_corner_x: Option<i32>,
    #[crudcrate(sortable, filterable)]
    pub upper_left_corner_y: Option<i32>,
    #[crudcrate(sortable, filterable)]
    pub lower_right_corner_x: Option<i32>,
    #[crudcrate(sortable, filterable)]
    pub lower_right_corner_y: Option<i32>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub last_updated: DateTime<Utc>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], use_target_models)]
    pub probe_locations: Vec<crate::tray_configurations::probes::models::Probe>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::tray_configurations::models::Entity",
        from = "Column::TrayConfigurationId",
        to = "crate::tray_configurations::models::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    TrayConfigurations,
    #[sea_orm(has_many = "crate::tray_configurations::wells::models::Entity")]
    Wells,
    #[sea_orm(has_many = "crate::tray_configurations::probes::models::Entity")]
    Probes,
}

impl Related<crate::tray_configurations::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TrayConfigurations.def()
    }
}

impl Related<crate::tray_configurations::wells::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Wells.def()
    }
}

impl Related<crate::tray_configurations::probes::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Probes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
