use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "wells")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tray_id: Uuid,
    pub row_label: String,
    pub column_number: i32,
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::routes::inp::db::Entity")]
    FreezingResults,
    #[sea_orm(
        belongs_to = "crate::routes::trays::db::Entity",
        from = "Column::TrayId",
        to = "crate::routes::trays::db::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Trays,
    #[sea_orm(has_many = "crate::routes::temperatures::wells::db::Entity")]
    WellTemperatures,
}

impl Related<crate::routes::inp::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FreezingResults.def()
    }
}

impl Related<crate::routes::trays::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Trays.def()
    }
}

impl Related<crate::routes::temperatures::wells::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WellTemperatures.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
