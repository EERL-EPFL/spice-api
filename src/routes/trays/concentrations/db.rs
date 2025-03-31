use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "inp_concentrations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub region_id: Uuid,
    pub temperature_celsius: Option<Decimal>,
    pub nm_value: Option<Decimal>,
    pub error: Option<Decimal>,
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
}

impl Related<crate::routes::trays::regions::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Regions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
