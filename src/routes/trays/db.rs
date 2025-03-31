use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "trays")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub experiment_id: Uuid,
    pub tray_number: Option<i32>,
    pub n_rows: Option<i32>,
    pub n_columns: Option<i32>,
    pub well_relative_diameter: Option<Decimal>,
    pub upper_left_corner_x: Option<i32>,
    pub upper_left_corner_y: Option<i32>,
    pub lower_right_corner_x: Option<i32>,
    pub lower_right_corner_y: Option<i32>,
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
    #[sea_orm(has_many = "crate::routes::trays::wells::db::Entity")]
    Wells,
}

impl Related<crate::routes::experiments::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Experiments.def()
    }
}

impl Related<crate::routes::trays::wells::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Wells.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
