use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "campaign")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub name: String,
    #[sea_orm(column_type = "Decimal(Some((9, 6)))", nullable)]
    pub latitude: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((9, 6)))", nullable)]
    pub longitude: Option<Decimal>,
    #[sea_orm(column_type = "Text", nullable)]
    pub comment: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::routes::samples::db::Entity")]
    Samples,
    #[sea_orm(has_many = "crate::routes::experiments::db::Entity")]
    Experiments,
}

impl Related<crate::routes::samples::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Samples.def()
    }
}
impl Related<crate::routes::experiments::db::Entity> for Entity {
    fn to() -> RelationDef {
        crate::routes::samples::db::Relation::Experiments.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            crate::routes::samples::db::Relation::Experiments
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}
