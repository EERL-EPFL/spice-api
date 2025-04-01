use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "treatments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub experiment_id: Uuid,
    #[sea_orm(column_type = "Text", nullable)]
    pub treatment_code: Option<String>,
    pub dilution_factor: Option<Decimal>,
    #[sea_orm(column_type = "Text", nullable)]
    pub notes: Option<String>,
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::routes::experiments::db::Entity",
        from = "Column::ExperimentId",
        to = "crate::routes::experiments::db::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Experiments,
    #[sea_orm(has_many = "crate::routes::trays::regions::db::Entity")]
    Regions,
}

impl Related<crate::routes::experiments::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Experiments.def()
    }
}

impl Related<crate::routes::trays::regions::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Regions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
