use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "well_temperatures")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub well_id: Uuid,
    pub timestamp: Option<DateTimeWithTimeZone>,
    pub temperature_celsius: Option<Decimal>,
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::routes::trays::wells::db::Entity",
        from = "Column::WellId",
        to = "crate::routes::trays::wells::db::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Wells,
}

impl Related<crate::routes::trays::wells::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Wells.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
