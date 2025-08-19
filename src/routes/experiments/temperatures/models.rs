use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "temperature_readings")]
#[crudcrate(
    generate_router,
    api_struct = "TemperatureReading",
    name_singular = "temperature_reading",
    name_plural = "temperature_readings",
    description = "Temperature readings from multiple probes at specific timestamps during experiments."
)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub experiment_id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub timestamp: DateTime<Utc>,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub image_filename: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub probe_1: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub probe_2: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub probe_3: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub probe_4: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub probe_5: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub probe_6: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub probe_7: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub probe_8: Option<Decimal>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = None)]
    pub average: Option<Decimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::routes::experiments::models::Entity",
        from = "Column::ExperimentId",
        to = "crate::routes::experiments::models::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Experiments,
    #[sea_orm(has_many = "crate::routes::experiments::phase_transitions::models::Entity")]
    WellPhaseTransitions,
}

impl Related<crate::routes::experiments::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Experiments.def()
    }
}

impl Related<crate::routes::experiments::phase_transitions::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WellPhaseTransitions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
