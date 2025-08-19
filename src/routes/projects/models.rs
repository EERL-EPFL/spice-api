use crate::routes::locations::models::Location;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "projects")]
#[crudcrate(
    generate_router,
    api_struct = "Project",
    name_singular = "project",
    name_plural = "projects",
    description = "Projects provide a way to organise locations hierarchically. Each project can contain multiple locations and provides visual organization through color coding.",
    fn_get_one = get_one,
)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[sea_orm(unique)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub name: String,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext, list_model = false)]
    pub note: Option<String>,
    #[crudcrate(sortable, filterable, fulltext)]
    pub colour: Option<String>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub last_updated: DateTime<Utc>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], list_model = false)]
    pub locations: Vec<Location>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::routes::locations::models::Entity")]
    Locations,
}

impl Related<crate::routes::locations::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Locations.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

async fn get_one(db: &DatabaseConnection, id: Uuid) -> Result<Project, DbErr> {
    let model = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Project not found".to_string()))?;

    let locations: Vec<crate::routes::locations::models::Location> = model
        .find_related(crate::routes::locations::models::Entity)
        .all(db)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    let mut project: Project = model.into();
    project.locations = locations;
    Ok(project)
}
