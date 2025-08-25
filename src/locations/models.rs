use crate::services::convex_hull_service;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels};
use sea_orm::entity::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "locations")]
#[crudcrate(
    generate_router,
    api_struct = "Location",
    name_singular = "location",
    name_plural = "locations",
    description = "Locations represent physical places where experiments are conducted. Each location belongs to a project and can contain multiple samples and experiments.",
    fn_get_one = get_one_location,
    fn_get_all = get_all_locations,
)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[sea_orm(unique)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub name: String,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub comment: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub project_id: Option<Uuid>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable)]
    pub last_updated: DateTime<Utc>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = None, create_model = false, update_model = false)]
    pub area: Option<serde_json::Value>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = None, create_model = false, update_model = false)]
    pub color: Option<String>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = None, create_model = false, update_model = false)]
    pub project_name: Option<String>,
    // Removed embedded samples and experiments to prevent circular dependency
    // Use /experiments/{id}/samples and /experiments/{id}/locations endpoints instead
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::projects::models::Entity",
        from = "Column::ProjectId",
        to = "crate::projects::models::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    Projects,
    #[sea_orm(has_many = "crate::samples::models::Entity")]
    Samples,
}

impl Related<crate::projects::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Projects.def()
    }
}

impl Related<crate::samples::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Samples.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// Custom `get_one` that loads area (convex hull) and project info only
/// Removed samples/experiments loading to prevent circular dependency
async fn get_one_location(db: &DatabaseConnection, id: Uuid) -> Result<Location, DbErr> {
    let location_model = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Location not found".to_string()))?;

    // Get area (convex hull, returns None for SQLite)
    // Use buffered convex hull to ensure we always get a polygon (not just a point)
    let area = convex_hull_service::get_location_convex_hull_buffered(db, id, 50.0).await;

    // Get project color and name if project_id exists
    let (project_color, project_name) = if let Some(project_id) = location_model.project_id {
        if let Some(project) = crate::projects::models::Entity::find_by_id(project_id)
            .one(db)
            .await? {
            (project.colour, Some(project.name))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let mut location: Location = location_model.into();
    location.area = area;
    location.color = project_color;
    location.project_name = project_name;
    // Note: samples and experiments removed to prevent circular dependency
    // Use /experiments/{id}/samples and /experiments/{id}/locations endpoints instead

    Ok(location)
}

/// Custom `get_all` that includes area (convex hull) for each location
async fn get_all_locations(
    db: &DatabaseConnection,
    condition: &sea_orm::Condition,
    order_column: Column,
    order_direction: sea_orm::Order,
    offset: u64,
    limit: u64,
) -> Result<Vec<LocationList>, DbErr> {
    let models = Entity::find()
        .filter(condition.clone())
        .order_by(order_column, order_direction)
        .offset(offset)
        .limit(limit)
        .all(db)
        .await?;

    // For each location, fetch the convex hull area and project color
    let mut locations: Vec<LocationList> = Vec::new();

    for model in models {
        // Get area (convex hull, returns None for SQLite)
        // Use buffered convex hull to ensure we always get a polygon (not just a point)
        let area = convex_hull_service::get_location_convex_hull_buffered(db, model.id, 50.0).await;

        // Get project color and name if project_id exists
        let (project_color, project_name) = if let Some(project_id) = model.project_id {
            if let Some(project) = crate::projects::models::Entity::find_by_id(project_id)
                .one(db)
                .await? {
                (project.colour, Some(project.name))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // Convert model to LocationList and attach area, color, and project name
        let mut location_list = LocationList::from(model);
        location_list.area = area;
        location_list.color = project_color;
        location_list.project_name = project_name;
        locations.push(location_list);
    }

    Ok(locations)
}
