use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "regions")]
#[crudcrate(
    generate_router,
    api_struct = "Region",
    name_singular = "region",
    name_plural = "regions",
    description = "Regions define coordinate-based areas within trays for organizing experimental treatments."
)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[crudcrate(sortable, filterable, create_model = false)]
    pub experiment_id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub treatment_id: Option<Uuid>,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub name: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable)]
    pub display_colour_hex: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub tray_id: Option<i32>,
    #[crudcrate(sortable, filterable)]
    pub col_min: Option<i32>,
    #[crudcrate(sortable, filterable)]
    pub row_min: Option<i32>,
    #[crudcrate(sortable, filterable)]
    pub col_max: Option<i32>,
    #[crudcrate(sortable, filterable)]
    pub row_max: Option<i32>,
    #[crudcrate(sortable, filterable)]
    pub dilution_factor: Option<i32>,
    #[crudcrate(filterable)]
    pub is_background_key: bool,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub last_updated: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::experiments::models::Entity",
        from = "Column::ExperimentId",
        to = "crate::experiments::models::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Experiments,
    #[sea_orm(
        belongs_to = "crate::treatments::models::Entity",
        from = "Column::TreatmentId",
        to = "crate::treatments::models::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Treatments,
}

impl Related<crate::experiments::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Experiments.def()
    }
}

impl Related<crate::treatments::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Treatments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
