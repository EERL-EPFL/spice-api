use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "experiments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub campaign_id: Option<Uuid>,
    pub created_by: Option<String>,
    pub experiment_date: Option<DateTime<Utc>>,
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub temperature_ramp: Option<f64>,
    pub temperature_start: Option<f64>,
    pub temperature_end: Option<f64>,
    pub is_calibration: bool,
    pub remarks: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::routes::campaigns::db::Entity",
        from = "Column::CampaignId",
        to = "crate::routes::campaigns::db::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Campaign,
    #[sea_orm(has_many = "crate::routes::inp::configs::db::Entity")]
    Configs,
    #[sea_orm(has_many = "crate::routes::experiments::images::db::Entity")]
    Images,
    #[sea_orm(has_many = "crate::routes::trays::regions::db::Entity")]
    Regions,
    #[sea_orm(has_many = "crate::routes::s3::db::Entity")]
    S3Assets,
    #[sea_orm(has_one = "crate::routes::samples::db::Entity")]
    Samples,
    #[sea_orm(has_many = "crate::routes::temperatures::probes::db::Entity")]
    TemperatureProbes,
    #[sea_orm(has_many = "crate::routes::trays::db::Entity")]
    Trays,
    #[sea_orm(has_many = "crate::routes::samples::treatments::db::Entity")]
    Treatments,
}

impl Related<crate::routes::campaigns::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Campaign.def()
    }
}

impl Related<crate::routes::inp::configs::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Configs.def()
    }
}

impl Related<crate::routes::experiments::images::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Images.def()
    }
}

impl Related<crate::routes::trays::regions::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Regions.def()
    }
}

impl Related<crate::routes::s3::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::S3Assets.def()
    }
}

impl Related<crate::routes::samples::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Samples.def()
    }
}

impl Related<crate::routes::temperatures::probes::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TemperatureProbes.def()
    }
}

impl Related<crate::routes::trays::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Trays.def()
    }
}

impl Related<crate::routes::samples::treatments::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Treatments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
