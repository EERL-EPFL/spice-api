use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "treatments")]
pub struct Model {
    #[sea_orm(column_type = "Text", nullable)]
    pub notes: Option<String>,
    pub sample_id: Option<Uuid>,
    pub last_updated: DateTimeWithTimeZone,
    pub created_at: DateTimeWithTimeZone,
    #[sea_orm(column_type = "Decimal(Some((16, 10)))", nullable)]
    pub enzyme_volume_litres: Option<Decimal>,
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: TreatmentName,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::routes::trays::regions::models::Entity")]
    Regions,
    #[sea_orm(
        belongs_to = "crate::routes::samples::models::Entity",
        from = "Column::SampleId",
        to = "crate::routes::samples::models::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Samples,
}

impl Related<crate::routes::trays::regions::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Regions.def()
    }
}

impl Related<crate::routes::samples::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Samples.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, ToSchema, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "treatment_name")]
pub enum TreatmentName {
    #[sea_orm(string_value = "none")]
    #[serde(rename = "none")]
    None,
    #[sea_orm(string_value = "heat")]
    #[serde(rename = "heat")]
    Heat,
    #[sea_orm(string_value = "h2o2")]
    #[serde(rename = "h2o2")]
    H2o2,
}
