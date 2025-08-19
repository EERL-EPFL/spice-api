use crate::services::convex_hull_service;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels};
use sea_orm::entity::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, Statement};

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
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = None, list_model = false, create_model = false, update_model = false)]
    pub samples: Option<Vec<crate::samples::models::Sample>>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = None, list_model = false, create_model = false, update_model = false)]
    pub experiments: Option<Vec<crate::experiments::models::Experiment>>,
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

/// Custom `get_one` that loads area (convex hull), related samples with treatments, and associated experiments
async fn get_one_location(db: &DatabaseConnection, id: Uuid) -> Result<Location, DbErr> {
    let model = Entity::find_by_id(id)
        .find_with_related(crate::samples::models::Entity)
        .all(db)
        .await?;

    let (location_model, samples) = model
        .into_iter()
        .next()
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

    // Get associated experiments via samples -> treatments -> regions -> experiments
    let experiments_query = r"
        SELECT DISTINCT e.*
        FROM experiments e
        JOIN regions r ON r.experiment_id = e.id
        JOIN treatments t ON t.id = r.treatment_id
        JOIN samples s ON s.id = t.sample_id
        WHERE s.location_id = $1
        ORDER BY e.performed_at DESC
    ";

    let experiments: Vec<crate::experiments::models::Model> =
        crate::experiments::models::Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                db.get_database_backend(),
                experiments_query,
                vec![id.into()],
            ))
            .all(db)
            .await?;

    // Load treatments for each sample
    let mut enriched_samples = Vec::new();
    for sample in samples {
        let treatments = crate::treatments::models::Entity::find()
            .filter(crate::treatments::models::Column::SampleId.eq(sample.id))
            .all(db)
            .await?;

        let mut sample_with_treatments: crate::samples::models::Sample = sample.into();
        sample_with_treatments.treatments = treatments.into_iter().map(std::convert::Into::into).collect();
        enriched_samples.push(sample_with_treatments);
    }

    let mut location: Location = location_model.into();
    location.area = area;
    location.color = project_color;
    location.project_name = project_name;
    location.samples = Some(enriched_samples);
    location.experiments = Some(experiments.into_iter().map(std::convert::Into::into).collect());

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
