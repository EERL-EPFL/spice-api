use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, ToSchema, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "sample_type")]
pub enum SampleType {
    #[sea_orm(string_value = "bulk")]
    #[serde(rename = "bulk")]
    Bulk,
    #[sea_orm(string_value = "filter")]
    #[serde(rename = "filter")]
    Filter,
    #[sea_orm(string_value = "procedural_blank")]
    #[serde(rename = "procedural_blank")]
    ProceduralBlank,
    #[sea_orm(string_value = "pure_water")]
    #[serde(rename = "pure_water")]
    PureWater,
}
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "samples")]
pub struct Model {
    #[sea_orm(column_type = "Text")]
    pub name: String,
    pub start_time: Option<DateTimeWithTimeZone>,
    pub stop_time: Option<DateTimeWithTimeZone>,
    #[sea_orm(column_type = "Decimal(Some((16, 10)))", nullable)]
    pub flow_litres_per_minute: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((16, 10)))", nullable)]
    pub total_volume: Option<Decimal>,
    #[sea_orm(column_type = "Text", nullable)]
    pub material_description: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub extraction_procedure: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub filter_substrate: Option<String>,
    pub suspension_volume_litres: Option<Decimal>,
    pub air_volume_litres: Option<Decimal>,
    pub water_volume_litres: Option<Decimal>,
    pub initial_concentration_gram_l: Option<Decimal>,
    pub well_volume_litres: Option<Decimal>,
    #[sea_orm(column_type = "Text", nullable)]
    pub remarks: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((9, 6)))", nullable)]
    pub longitude: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((9, 6)))", nullable)]
    pub latitude: Option<Decimal>,
    pub location_id: Option<Uuid>,
    pub created_at: DateTimeWithTimeZone,
    pub last_updated: DateTimeWithTimeZone,
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub r#type: SampleType,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::routes::locations::models::Entity",
        from = "Column::LocationId",
        to = "crate::routes::locations::models::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Locations,
    #[sea_orm(has_many = "crate::routes::treatments::models::Entity")]
    Treatments,
}

impl Related<crate::routes::locations::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Locations.def()
    }
}

impl Related<crate::routes::treatments::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Treatments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
