use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

// Simplified tray assignment structure - tray details directly embedded
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct TrayAssignment {
    pub order_sequence: i32,
    pub rotation_degrees: i32,
    // Tray details directly in the assignment (flattened structure)
    pub name: Option<String>,
    pub qty_x_axis: Option<i32>,
    pub qty_y_axis: Option<i32>,
    pub well_relative_diameter: Option<Decimal>,
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
    pub trays: Vec<TrayAssignment>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![])]
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

// Extended TrayConfiguration for API responses that includes nested data
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct TrayConfigurationExtended {
    pub id: Uuid,
    pub name: Option<String>,
    pub experiment_default: bool,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub trays: Vec<TrayAssignment>,
    pub associated_experiments: Vec<serde_json::Value>,
}

// Convert from basic TrayConfiguration to Extended version
impl From<TrayConfiguration> for TrayConfigurationExtended {
    fn from(basic: TrayConfiguration) -> Self {
        Self {
            id: basic.id,
            name: basic.name,
            experiment_default: basic.experiment_default,
            created_at: basic.created_at,
            last_updated: basic.last_updated,
            trays: basic.trays,
            associated_experiments: basic.associated_experiments,
        }
    }
}

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

    let mut tray_assignments: Vec<TrayAssignment> = assignments
        .into_iter()
        .map(|assignment| TrayAssignment {
            order_sequence: assignment.order_sequence,
            rotation_degrees: assignment.rotation_degrees,
            name: assignment.name,
            qty_x_axis: assignment.qty_x_axis,
            qty_y_axis: assignment.qty_y_axis,
            well_relative_diameter: assignment.well_relative_diameter,
        })
        .collect();

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
    tray_config.trays = tray_assignments;
    tray_config.associated_experiments = experiments;

    Ok(tray_config)
}

// Custom crudcrate create function to handle nested tray assignments
pub async fn create_tray_configuration(
    db: &DatabaseConnection,
    data: TrayConfigurationCreate,
) -> Result<TrayConfiguration, DbErr> {
    use sea_orm::{Set, TransactionTrait, prelude::Expr};

    // Validate tray data before creating (simplified - all data is in assignment)
    for assignment in &data.trays {
        if let Some(qty_x) = assignment.qty_x_axis {
            if qty_x < 1 {
                return Err(DbErr::Custom(
                    "Validation failed: qty_x_axis must be positive".to_string(),
                ));
            }
        }
        if let Some(qty_y) = assignment.qty_y_axis {
            if qty_y < 1 {
                return Err(DbErr::Custom(
                    "Validation failed: qty_y_axis must be positive".to_string(),
                ));
            }
        }
        if let Some(diameter) = &assignment.well_relative_diameter {
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

    // Handle tray assignments (simplified - create directly in trays table)
    for assignment in data.trays {
        let tray_active = crate::routes::tray_configurations::trays::models::ActiveModel {
            id: Set(Uuid::new_v4()),
            tray_configuration_id: Set(tray_config_id),
            order_sequence: Set(assignment.order_sequence),
            rotation_degrees: Set(assignment.rotation_degrees),
            name: Set(assignment.name),
            qty_x_axis: Set(assignment.qty_x_axis),
            qty_y_axis: Set(assignment.qty_y_axis),
            well_relative_diameter: Set(assignment.well_relative_diameter),
            created_at: Set(chrono::Utc::now()),
            last_updated: Set(chrono::Utc::now()),
        };
        tray_active.insert(&txn).await?;
    }

    txn.commit().await?;

    // Return the complete tray configuration with nested data
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

        // Create new assignments (simplified - create directly in trays table)
        for assignment in trays {
            let tray_active = crate::routes::tray_configurations::trays::models::ActiveModel {
                id: Set(Uuid::new_v4()),
                tray_configuration_id: Set(id),
                order_sequence: Set(assignment.order_sequence),
                rotation_degrees: Set(assignment.rotation_degrees),
                name: Set(assignment.name),
                qty_x_axis: Set(assignment.qty_x_axis),
                qty_y_axis: Set(assignment.qty_y_axis),
                well_relative_diameter: Set(assignment.well_relative_diameter),
                created_at: Set(chrono::Utc::now()),
                last_updated: Set(chrono::Utc::now()),
            };
            tray_active.insert(&txn).await?;
        }
    }

    txn.commit().await?;

    // Return the complete tray configuration with nested data
    get_one_tray_configuration(db, id).await
}

// Convert from Extended back to basic
impl From<TrayConfigurationExtended> for TrayConfiguration {
    fn from(extended: TrayConfigurationExtended) -> Self {
        Self {
            id: extended.id,
            name: extended.name,
            experiment_default: extended.experiment_default,
            created_at: extended.created_at,
            last_updated: extended.last_updated,
            trays: extended.trays,
            associated_experiments: extended.associated_experiments,
        }
    }
}
