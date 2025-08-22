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
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = None)]
    pub average: Option<Decimal>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], list_model=false)]
    pub probe_readings: Vec<crate::experiments::probe_temperature_readings::models::ProbeTemperatureReading>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::experiments::models::Entity",
        from = "Column::ExperimentId",
        to = "crate::experiments::models::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Experiments,
    #[sea_orm(has_many = "crate::experiments::phase_transitions::models::Entity")]
    WellPhaseTransitions,
    #[sea_orm(has_many = "crate::experiments::probe_temperature_readings::models::Entity")]
    ProbeTemperatureReadings,
}

impl Related<crate::experiments::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Experiments.def()
    }
}

impl Related<crate::experiments::phase_transitions::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WellPhaseTransitions.def()
    }
}

impl Related<crate::experiments::probe_temperature_readings::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProbeTemperatureReadings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
