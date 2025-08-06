use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "experiments")]
pub struct Model {
    #[sea_orm(column_type = "Text", unique)]
    pub name: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub username: Option<String>,
    pub performed_at: Option<DateTimeWithTimeZone>,
    pub temperature_ramp: Option<Decimal>,
    pub temperature_start: Option<Decimal>,
    pub temperature_end: Option<Decimal>,
    pub is_calibration: bool,
    #[sea_orm(column_type = "Text", nullable)]
    pub remarks: Option<String>,
    pub tray_configuration_id: Option<Uuid>,
    pub created_at: DateTimeWithTimeZone,
    pub last_updated: DateTimeWithTimeZone,
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::routes::trays::regions::models::Entity")]
    Regions,
    #[sea_orm(has_many = "crate::routes::assets::models::Entity")]
    S3Assets,
    #[sea_orm(has_many = "crate::routes::experiments::temperatures::models::Entity")]
    TemperatureReadings,
    #[sea_orm(
        belongs_to = "crate::routes::trays::configurations::models::Entity",
        from = "Column::TrayConfigurationId",
        to = "crate::routes::trays::configurations::models::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    TrayConfigurations,
    #[sea_orm(has_many = "crate::routes::experiments::phase_transitions::models::Entity")]
    WellPhaseTransitions,
}

impl Related<crate::routes::trays::regions::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Regions.def()
    }
}

impl Related<crate::routes::assets::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::S3Assets.def()
    }
}

impl Related<crate::routes::experiments::temperatures::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TemperatureReadings.def()
    }
}

impl Related<crate::routes::trays::configurations::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TrayConfigurations.def()
    }
}

impl Related<crate::routes::experiments::phase_transitions::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WellPhaseTransitions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
