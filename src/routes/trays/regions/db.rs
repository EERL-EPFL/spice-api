use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "regions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub experiment_id: Uuid,
    #[sea_orm(column_type = "Text", nullable)]
    pub region_name: Option<String>,
    pub treatment_id: Option<Uuid>,
    pub wells: Option<Vec<Uuid>>,
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
    #[sea_orm(has_many = "crate::routes::inp::db::Entity")]
    FreezingResults,
    #[sea_orm(has_many = "crate::routes::trays::concentrations::db::Entity")]
    InpConcentrations,
    #[sea_orm(
        belongs_to = "crate::routes::samples::treatments::db::Entity",
        from = "Column::TreatmentId",
        to = "crate::routes::samples::treatments::db::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Treatments,
}

impl Related<crate::routes::experiments::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Experiments.def()
    }
}

impl Related<crate::routes::inp::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FreezingResults.def()
    }
}

impl Related<crate::routes::trays::concentrations::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InpConcentrations.def()
    }
}

impl Related<crate::routes::samples::treatments::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Treatments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
