use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use uuid::Uuid;

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
    pub trays: Vec<serde_json::Value>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], list_model=false)]
    pub associated_experiments: Vec<serde_json::Value>,
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
    // First get the basic model
    let model: Model = match Entity::find_by_id(id).one(db).await? {
        Some(model) => model,
        None => {
            return Err(DbErr::RecordNotFound(
                "tray_configuration not found".to_string(),
            ));
        }
    };

    // Load tray assignments (now simplified - all data is in the trays table)
    let assignments = crate::routes::tray_configurations::trays::models::Entity::find()
        .filter(
            crate::routes::tray_configurations::trays::models::Column::TrayConfigurationId.eq(id),
        )
        .all(db)
        .await?;

    let mut tray_assignments: Vec<crate::routes::tray_configurations::trays::models::Model> =
        assignments;

    // Sort by order_sequence as tests expect
    tray_assignments.sort_by_key(|a| a.order_sequence);

    // Load associated experiments
    let experiments: Vec<serde_json::Value> = crate::routes::experiments::models::Entity::find()
        .filter(crate::routes::experiments::models::Column::TrayConfigurationId.eq(id))
        .all(db)
        .await?
        .into_iter()
        .map(|exp| {
            serde_json::json!({
                "id": exp.id,
                "name": exp.name,
                "username": exp.username,
                "remarks": exp.remarks
            })
        })
        .collect();

    // Convert to crudcrate-generated TrayConfiguration and populate non-db fields
    let mut tray_config: TrayConfiguration = model.into();
    tray_config.trays = tray_assignments.into_iter().map(|t| serde_json::to_value(t).unwrap()).collect();
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
    let models = Entity::find()
        .filter(condition.clone())
        .order_by(order_column, order_direction)
        .offset(offset)
        .limit(limit)
        .all(db)
        .await?;

    if models.is_empty() {
        return Ok(vec![]);
    }

    let model_ids: Vec<Uuid> = models.iter().map(|m| m.id).collect();

    // Load all tray assignments for these configurations
    let all_assignments = crate::routes::tray_configurations::trays::models::Entity::find()
        .filter(
            crate::routes::tray_configurations::trays::models::Column::TrayConfigurationId
                .is_in(model_ids.clone()),
        )
        .all(db)
        .await?;

    // Group assignments by tray_configuration_id
    let mut assignments_map: std::collections::HashMap<
        Uuid,
        Vec<crate::routes::tray_configurations::trays::models::Model>,
    > = std::collections::HashMap::new();

    for assignment in all_assignments {
        let tray_config_id = assignment.tray_configuration_id;
        assignments_map
            .entry(tray_config_id)
            .or_insert_with(Vec::new)
            .push(assignment);
    }

    // Sort assignments within each group by order_sequence
    for assignments in assignments_map.values_mut() {
        assignments.sort_by_key(|a| a.order_sequence);
    }

    // Convert models to TrayConfiguration and populate nested data
    let mut tray_configs: Vec<TrayConfigurationList> = models
        .into_iter()
        .map(|model| {
            let mut tray_config: TrayConfigurationList = model.into();
            let config_id = tray_config.id;

            // Set trays for this configuration
            tray_config.trays = assignments_map.remove(&config_id).unwrap_or_default().into_iter().map(|t| serde_json::to_value(t).unwrap()).collect();

            tray_config
        })
        .collect();

    Ok(tray_configs)
}

// Custom crudcrate create function to handle nested tray assignments
pub async fn create_tray_configuration(
    db: &DatabaseConnection,
    data: TrayConfigurationCreate,
) -> Result<TrayConfiguration, DbErr> {
    use sea_orm::{Set, TransactionTrait, prelude::Expr};

    // Deserialize and validate tray data before creating
    let mut tray_data_vec = Vec::new();
    for tray_json in &data.trays {
        let tray_data: crate::routes::tray_configurations::trays::models::TrayCreateInput = serde_json::from_value(tray_json.clone())
            .map_err(|e| DbErr::Custom(format!("Invalid tray data: {}", e)))?;
        tray_data_vec.push(tray_data);
    }
    
    for tray in &tray_data_vec {
        if let Some(qty_x) = tray.qty_x_axis {
            if qty_x < 1 {
                return Err(DbErr::Custom(
                    "Validation failed: qty_x_axis must be positive".to_string(),
                ));
            }
        }
        if let Some(qty_y) = tray.qty_y_axis {
            if qty_y < 1 {
                return Err(DbErr::Custom(
                    "Validation failed: qty_y_axis must be positive".to_string(),
                ));
            }
        }
        if let Some(diameter) = &tray.well_relative_diameter {
            if diameter.is_sign_negative() || *diameter == rust_decimal::Decimal::ZERO {
                return Err(DbErr::Custom(
                    "Validation failed: well_relative_diameter must be positive".to_string(),
                ));
            }
        }
    }

    let txn = db.begin().await?;

    // If experiment_default is true, set all others to false
    if data.experiment_default {
        Entity::update_many()
            .col_expr(Column::ExperimentDefault, Expr::value(false))
            .exec(&txn)
            .await?;
    }

    // Create the main tray configuration
    let tray_config_id = Uuid::new_v4();
    let active_model = ActiveModel {
        id: Set(tray_config_id),
        name: Set(data.name.clone()),
        experiment_default: Set(data.experiment_default),
        created_at: Set(chrono::Utc::now()),
        last_updated: Set(chrono::Utc::now()),
    };
    active_model.insert(&txn).await?;

    // Handle tray assignments (create directly in trays table)
    for tray in tray_data_vec {
        let tray_active = crate::routes::tray_configurations::trays::models::ActiveModel {
            id: Set(Uuid::new_v4()),
            tray_configuration_id: Set(tray_config_id),
            order_sequence: Set(tray.order_sequence),
            rotation_degrees: Set(tray.rotation_degrees),
            name: Set(tray.name),
            qty_x_axis: Set(tray.qty_x_axis),
            qty_y_axis: Set(tray.qty_y_axis),
            well_relative_diameter: Set(tray.well_relative_diameter),
            created_at: Set(chrono::Utc::now()),
            last_updated: Set(chrono::Utc::now()),
        };
        tray_active.insert(&txn).await?;
    }

    txn.commit().await?;

    // Return the complete tray configuration with nested data using custom get_one function
    get_one_tray_configuration(db, tray_config_id).await
}

// Custom crudcrate update function to handle nested tray assignments
pub async fn update_tray_configuration(
    db: &DatabaseConnection,
    id: Uuid,
    update_data: TrayConfigurationUpdate,
) -> Result<TrayConfiguration, DbErr> {
    use crudcrate::traits::MergeIntoActiveModel;
    use sea_orm::{Set, TransactionTrait, prelude::Expr};

    let txn = db.begin().await?;

    // If experiment_default is true, set all others to false
    if let Some(Some(experiment_default)) = update_data.experiment_default {
        if experiment_default {
            Entity::update_many()
                .col_expr(Column::ExperimentDefault, Expr::value(false))
                .filter(Column::Id.ne(id))
                .exec(&txn)
                .await?;
        }
    }

    // Update the main tray configuration
    let existing: ActiveModel = Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or(DbErr::RecordNotFound(
            "tray_configuration not found".to_string(),
        ))?
        .into();

    let trays = update_data.trays.clone();
    let updated_model =
        <TrayConfigurationUpdate as MergeIntoActiveModel<ActiveModel>>::merge_into_activemodel(
            update_data,
            existing,
        )?;
    updated_model.update(&txn).await?;

    // Handle tray assignments update - simplified (just delete and recreate in trays table)
    if !trays.is_empty() {
        // Remove old assignments (simplified - everything is in one table now)
        crate::routes::tray_configurations::trays::models::Entity::delete_many()
            .filter(
                crate::routes::tray_configurations::trays::models::Column::TrayConfigurationId
                    .eq(id),
            )
            .exec(&txn)
            .await?;

        // Create new assignments (create directly in trays table)
        for tray_json in trays {
            let tray: crate::routes::tray_configurations::trays::models::TrayCreateInput = serde_json::from_value(tray_json)
                .map_err(|e| DbErr::Custom(format!("Invalid tray data: {}", e)))?;
            let tray_active = crate::routes::tray_configurations::trays::models::ActiveModel {
                id: Set(Uuid::new_v4()),
                tray_configuration_id: Set(id),
                order_sequence: Set(tray.order_sequence),
                rotation_degrees: Set(tray.rotation_degrees),
                name: Set(tray.name),
                qty_x_axis: Set(tray.qty_x_axis),
                qty_y_axis: Set(tray.qty_y_axis),
                well_relative_diameter: Set(tray.well_relative_diameter),
                created_at: Set(chrono::Utc::now()),
                last_updated: Set(chrono::Utc::now()),
            };
            tray_active.insert(&txn).await?;
        }
    }

    txn.commit().await?;

    // Return the complete tray configuration with nested data using custom get_one function
    get_one_tray_configuration(db, id).await
}
