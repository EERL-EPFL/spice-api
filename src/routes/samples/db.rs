use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "samples")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub experiment_id: Uuid,
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
}

impl Related<crate::routes::experiments::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Experiments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
