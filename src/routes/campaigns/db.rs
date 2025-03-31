// use crate::routes::areas::db::Entity as Area;
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "campaign")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub name: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub comment: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub last_updated: DateTime<Utc>,
    pub user_id: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // #[sea_orm(has_many = "Area")]
    // Area,
    // #[sea_orm(has_many = "crate::routes::instrument_experiments::db::Entity")]
    // Instrumentexperiment,
}

// impl Related<Area> for Entity {
//     fn to() -> RelationDef {
//         // Relation::Area.def()
//     }
// }

// impl Related<crate::routes::instrument_experiments::db::Entity> for Entity {
//     fn to() -> RelationDef {
//         // Relation::Instrumentexperiment.def()
//     }
// }

impl ActiveModelBehavior for ActiveModel {}
