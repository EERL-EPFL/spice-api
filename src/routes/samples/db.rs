use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "samples")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(column_type = "Text")]
    pub name: String,
    #[sea_orm(column_type = "Text")]
    pub r#type: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub treatment: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub material_description: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub extraction_procedure: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub filter_substrate: Option<String>,
    pub suspension_volume_liters: Option<Decimal>,
    pub air_volume_liters: Option<Decimal>,
    pub water_volume_liters: Option<Decimal>,
    pub initial_concentration_gram_l: Option<Decimal>,
    pub well_volume_liters: Option<Decimal>,
    #[sea_orm(column_type = "Text", nullable)]
    pub background_region_key: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub remarks: Option<String>,
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub campaign_id: Option<Uuid>,
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
    #[sea_orm(has_many = "crate::routes::experiments::db::Entity")]
    Experiments,
}

impl Related<crate::routes::experiments::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Experiments.def()
    }
}

impl Related<crate::routes::campaigns::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Campaign.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
