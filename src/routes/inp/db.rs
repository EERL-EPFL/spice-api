use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "freezing_results")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub well_id: Uuid,
    pub freezing_temperature_celsius: Option<Decimal>,
    pub is_frozen: Option<bool>,
    pub region_id: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::routes::trays::regions::db::Entity",
        from = "Column::RegionId",
        to = "crate::routes::trays::regions::db::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Regions,
    #[sea_orm(
        belongs_to = "crate::routes::trays::wells::db::Entity",
        from = "Column::WellId",
        to = "crate::routes::trays::wells::db::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Wells,
}

impl Related<crate::routes::trays::regions::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Regions.def()
    }
}

impl Related<crate::routes::trays::wells::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Wells.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
