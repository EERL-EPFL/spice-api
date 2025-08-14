use chrono::{DateTime, Utc};
use crudcrate::traits::MergeIntoActiveModel;
use crudcrate::{CRUDResource, EntityToModels};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::Expr;
use sea_orm::{QueryOrder, QuerySelect, Set};
use uuid::Uuid;

/// Input type for tray data in create/update requests (excludes server-managed fields)
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct TrayInput {
    pub order_sequence: i32,
    pub rotation_degrees: i32,
    pub name: Option<String>,
    pub qty_x_axis: Option<i32>,
    pub qty_y_axis: Option<i32>,
    pub well_relative_diameter: Option<Decimal>,
}

impl From<crate::routes::tray_configurations::trays::models::Model> for TrayInput {
    fn from(model: crate::routes::tray_configurations::trays::models::Model) -> Self {
        Self {
            order_sequence: model.order_sequence,
            rotation_degrees: model.rotation_degrees,
            name: model.name,
            qty_x_axis: model.qty_x_axis,
            qty_y_axis: model.qty_y_axis,
            well_relative_diameter: model.well_relative_diameter,
        }
    }
}
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "tray_configurations")]
#[crudcrate(
    generate_router,
    api_struct = "TrayConfiguration",
    name_singular = "tray_configuration",
    name_plural = "tray_configurations",
    description = "This endpoint manages tray configurations, which define the setup of trays used in experiments.",
    fn_get_one = get_one_tray_configuration,
    fn_get_all = get_all_tray_configurations,
    fn_create = create_tray_configuration,
    fn_update = update_tray_configuration,
    error_mapper = crudcrate::error_handling::BusinessErrorMapper
)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[sea_orm(column_type = "Text", nullable, unique)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub name: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub experiment_default: bool,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub last_updated: DateTime<Utc>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![])]
    pub trays: Vec<TrayInput>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], list_model=false)]
    pub associated_experiments: Vec<crate::routes::experiments::models::Experiment>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::routes::experiments::models::Entity")]
    Experiments,
    #[sea_orm(has_many = "crate::routes::tray_configurations::trays::models::Entity")]
    Trays,
}

impl Related<crate::routes::experiments::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Experiments.def()
    }
}

impl Related<crate::routes::tray_configurations::trays::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Trays.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// Custom crudcrate function to load nested tray assignments and experiments data
pub async fn get_one_tray_configuration(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<TrayConfiguration, DbErr> {
    // Use SeaORM to load model with related data
    let results = Entity::find_by_id(id)
        .find_with_related(crate::routes::tray_configurations::trays::models::Entity)
        .all(db)
        .await?;
    
    let (model, mut trays) = results.into_iter().next()
        .ok_or_else(|| DbErr::RecordNotFound("tray_configuration not found".to_string()))?;

    // Load associated experiments
    let experiments: Vec<crate::routes::experiments::models::Experiment> =
        crate::routes::experiments::models::Entity::find()
            .filter(crate::routes::experiments::models::Column::TrayConfigurationId.eq(id))
            .all(db)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();

    // Sort trays by order_sequence
    trays.sort_by_key(|t| t.order_sequence);

    // Convert to crudcrate-generated TrayConfiguration and populate non-db fields
    let mut tray_config: TrayConfiguration = model.into();
    tray_config.trays = trays.into_iter().map(Into::into).collect();
    tray_config.associated_experiments = experiments;

    Ok(tray_config)
}

// Custom crudcrate function to load all tray configurations with nested data
pub async fn get_all_tray_configurations(
    db: &DatabaseConnection,
    condition: &sea_orm::Condition,
    order_column: Column,
    order_direction: sea_orm::Order,
    offset: u64,
    limit: u64,
) -> Result<Vec<TrayConfigurationList>, DbErr> {
    // Use SeaORM's find_with_related to load trays in one query
    let models_with_trays = Entity::find()
        .filter(condition.clone())
        .order_by(order_column, order_direction)
        .offset(offset)
        .limit(limit)
        .find_with_related(crate::routes::tray_configurations::trays::models::Entity)
        .all(db)
        .await?;

    let tray_configs: Vec<TrayConfigurationList> = models_with_trays
        .into_iter()
        .map(|(model, mut trays)| {
            let mut tray_config: TrayConfigurationList = model.into();
            
            // Sort trays by order_sequence
            trays.sort_by_key(|t| t.order_sequence);
            
            // Convert trays to TrayList
            tray_config.trays = trays.into_iter().map(Into::into).collect();
            
            tray_config
        })
        .collect();

    Ok(tray_configs)
}

// Much simpler create function - just add to DB directly
pub async fn create_tray_configuration(
    db: &DatabaseConnection,
    data: TrayConfigurationCreate,
) -> Result<TrayConfiguration, DbErr> {
    // Simple validation
    for tray in &data.trays {
        if let Some(qty_x) = tray.qty_x_axis
            && qty_x < 1 {
                return Err(DbErr::Custom("qty_x_axis must be positive".to_string()));
            }
        if let Some(qty_y) = tray.qty_y_axis
            && qty_y < 1 {
                return Err(DbErr::Custom("qty_y_axis must be positive".to_string()));
            }
    }

    // If this is being set as experiment default, unset all other defaults first
    if data.experiment_default {
        Entity::update_many()
            .col_expr(Column::ExperimentDefault, Expr::value(false))
            .col_expr(Column::LastUpdated, Expr::value(chrono::Utc::now()))
            .exec(db)
            .await?;
    }

    // Create the main tray configuration
    let tray_config_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let tray_config_active = ActiveModel {
        id: Set(tray_config_id),
        name: Set(data.name.clone()),
        experiment_default: Set(data.experiment_default),
        created_at: Set(now),
        last_updated: Set(now),
    };
    tray_config_active.insert(db).await?;

    // Create individual trays
    for tray in &data.trays {
        let tray_active = crate::routes::tray_configurations::trays::models::ActiveModel {
            id: Set(Uuid::new_v4()),
            tray_configuration_id: Set(tray_config_id),
            order_sequence: Set(tray.order_sequence),
            rotation_degrees: Set(tray.rotation_degrees),
            name: Set(tray.name.clone()),
            qty_x_axis: Set(tray.qty_x_axis),
            qty_y_axis: Set(tray.qty_y_axis),
            well_relative_diameter: Set(tray.well_relative_diameter),
            created_at: Set(now),
            last_updated: Set(now),
        };
        tray_active.insert(db).await?;
    }

    // Return the complete configuration
    get_one_tray_configuration(db, tray_config_id).await
}

// Much simpler update function - just add to DB directly
pub async fn update_tray_configuration(
    db: &DatabaseConnection,
    id: Uuid,
    update_data: TrayConfigurationUpdate,
) -> Result<TrayConfiguration, DbErr> {
    // Simple validation for trays
    for tray in &update_data.trays {
        if let Some(qty_x) = tray.qty_x_axis
            && qty_x < 1 {
                return Err(DbErr::Custom("qty_x_axis must be positive".to_string()));
            }
        if let Some(qty_y) = tray.qty_y_axis
            && qty_y < 1 {
                return Err(DbErr::Custom("qty_y_axis must be positive".to_string()));
            }
    }

    // If being set as experiment default, unset all other defaults first
    if update_data.experiment_default == Some(Some(true)) {
        Entity::update_many()
            .filter(Column::Id.ne(id)) // Don't update the current record
            .col_expr(Column::ExperimentDefault, Expr::value(false))
            .col_expr(Column::LastUpdated, Expr::value(chrono::Utc::now()))
            .exec(db)
            .await?;
    }

    // Update the main tray configuration
    let existing: ActiveModel = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("tray_configuration not found".to_string()))?
        .into();

    let trays = update_data.trays.clone();
    let updated_model = <TrayConfigurationUpdate as MergeIntoActiveModel<ActiveModel>>::merge_into_activemodel(
        update_data,
        existing,
    )?;
    updated_model.update(db).await?;

    // Update trays - delete and recreate
    if !trays.is_empty() {
        // Remove old trays
        crate::routes::tray_configurations::trays::models::Entity::delete_many()
            .filter(crate::routes::tray_configurations::trays::models::Column::TrayConfigurationId.eq(id))
            .exec(db)
            .await?;

        // Create new trays
        let now = chrono::Utc::now();
        for tray in trays {
            let tray_active = crate::routes::tray_configurations::trays::models::ActiveModel {
                id: Set(Uuid::new_v4()),
                tray_configuration_id: Set(id),
                order_sequence: Set(tray.order_sequence),
                rotation_degrees: Set(tray.rotation_degrees),
                name: Set(tray.name.clone()),
                qty_x_axis: Set(tray.qty_x_axis),
                qty_y_axis: Set(tray.qty_y_axis),
                well_relative_diameter: Set(tray.well_relative_diameter),
                created_at: Set(now),
                last_updated: Set(now),
            };
            tray_active.insert(db).await?;
        }
    }

    // Return the complete tray configuration
    get_one_tray_configuration(db, id).await
}
