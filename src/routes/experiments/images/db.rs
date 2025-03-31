//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.8

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "images")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub experiment_id: Uuid,
    #[sea_orm(column_type = "Text")]
    pub filename: String,
    pub timestamp: Option<DateTimeWithTimeZone>,
    pub order_index: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::experiments::Entity",
        from = "Column::ExperimentId",
        to = "super::experiments::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Experiments,
}

impl Related<super::experiments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Experiments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
