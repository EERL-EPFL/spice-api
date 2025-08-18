use super::temperatures::models::TemperatureReading;
use crate::routes::experiments::services::build_results_summary;
use chrono::{DateTime, Utc};
use crudcrate::traits::MergeIntoActiveModel;
use crudcrate::{CRUDResource, EntityToModels};
use rust_decimal::Decimal;
use sea_orm::QuerySelect;
use sea_orm::{
    ActiveValue::Set, Condition, ConnectionTrait, EntityTrait, Order, QueryOrder, TransactionTrait,
    entity::prelude::*,
};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "experiments")]
#[crudcrate(
    generate_router,
    api_struct = "Experiment",
    name_singular = "experiment",
    name_plural = "experiments",
    description = "Experiments track ice nucleation testing sessions with associated data and results.",
    fn_get_one = get_one_experiment,
    fn_create = create_experiment,
    fn_update = update_experiment,
    fn_get_all = get_all_experiments
)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[sea_orm(column_type = "Text", unique)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub name: String,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub username: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub performed_at: Option<DateTime<Utc>>,
    #[crudcrate(sortable, filterable, list_model = false)]
    pub temperature_ramp: Option<Decimal>,
    #[crudcrate(sortable, filterable, list_model = false)]
    pub temperature_start: Option<Decimal>,
    #[crudcrate(sortable, filterable, list_model = false)]
    pub temperature_end: Option<Decimal>,
    #[crudcrate(filterable)]
    pub is_calibration: bool,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext, list_model = false)]
    pub remarks: Option<String>,
    #[crudcrate(sortable, filterable, list_model = false)]
    pub tray_configuration_id: Option<Uuid>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub last_updated: DateTime<Utc>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], list_model=false)]
    pub assets: Vec<crate::routes::assets::models::Asset>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], list_model=false, use_target_models)]
    pub regions: Vec<crate::routes::tray_configurations::regions::models::Region>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = None, list_model=false)]
    pub results_summary: Option<super::models::ExperimentResultsSummary>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::routes::tray_configurations::regions::models::Entity")]
    Regions,
    #[sea_orm(has_many = "crate::routes::assets::models::Entity")]
    S3Assets,
    #[sea_orm(has_many = "crate::routes::experiments::temperatures::models::Entity")]
    TemperatureReadings,
    #[sea_orm(
        belongs_to = "crate::routes::tray_configurations::models::Entity",
        from = "Column::TrayConfigurationId",
        to = "crate::routes::tray_configurations::models::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    TrayConfigurations,
    #[sea_orm(has_many = "crate::routes::experiments::phase_transitions::models::Entity")]
    WellPhaseTransitions,
}

impl Related<crate::routes::tray_configurations::regions::models::Entity> for Entity {
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

impl Related<crate::routes::tray_configurations::models::Entity> for Entity {
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

#[derive(ToSchema, Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct WellSummary {
    pub row: i32,
    pub col: i32,
    pub coordinate: String, // e.g., "A1", "B2"
    pub first_phase_change_time: Option<DateTime<Utc>>,
    pub first_phase_change_seconds: Option<i64>, // seconds from experiment start
    pub first_phase_change_temperature_probes: Option<TemperatureReading>,
    pub final_state: Option<String>, // "frozen", "liquid", "no_data"
    pub image_filename_at_freeze: Option<String>, // Image filename at time of first phase change (without .jpg extension)
    pub image_asset_id: Option<Uuid>, // Asset ID for the image at freeze time (for secure viewing via /assets/{id}/view)
    pub tray_id: Option<String>,      // UUID of the tray
    pub tray_name: Option<String>,
    pub dilution_factor: Option<i32>,
    pub sample: Option<crate::routes::samples::models::Sample>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SampleResultsSummary {
    pub sample: crate::routes::samples::models::Sample,
    pub treatments: Vec<TreatmentResultsSummary>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TreatmentResultsSummary {
    pub treatment: crate::routes::treatments::models::Treatment,
    pub wells: Vec<WellSummary>,
    pub wells_frozen: usize,
    pub wells_liquid: usize,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ExperimentResultsSummary {
    pub total_wells: usize,
    pub wells_with_data: usize,
    pub wells_frozen: usize,
    pub wells_liquid: usize,
    pub total_time_points: usize,
    pub first_timestamp: Option<DateTime<Utc>>,
    pub last_timestamp: Option<DateTime<Utc>>,
    pub sample_results: Vec<SampleResultsSummary>,
}

pub(super) async fn get_one_experiment(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Experiment, DbErr> {
    let model = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("Experiment not found".to_string()))?;

    let s3_assets = model
        .find_related(crate::routes::assets::models::Entity)
        .all(db)
        .await?;

    let regions: Vec<crate::routes::tray_configurations::regions::models::Region> = model
        .find_related(crate::routes::tray_configurations::regions::models::Entity)
        .all(db)
        .await?
        .into_iter()
        .map(Into::into) // Direct conversion using EntityToModels
        .collect();

    // Build results summary
    let results_summary = build_results_summary(id, db).await?;

    let mut experiment: Experiment = model.into();
    experiment.assets = s3_assets.into_iter().map(Into::into).collect();
    experiment.regions = regions;
    experiment.results_summary = results_summary;

    Ok(experiment)
}

pub(super) async fn create_experiment(
    db: &DatabaseConnection,
    data: ExperimentCreate,
) -> Result<Experiment, DbErr> {
    let txn = db.begin().await?;

    // Store regions before conversion since they're not part of the DB model
    let regions_to_create = data.regions.clone();

    // Create the experiment first (avoid data.into() due to non-db attributes)
    // Manually construct ActiveModel from database fields only
    let mut experiment_model = ActiveModel::new();
    experiment_model.id = Set(Uuid::new_v4()); // Explicitly set UUID for SQLite compatibility
    experiment_model.name = Set(data.name);
    if let Some(username) = data.username {
        experiment_model.username = Set(Some(username));
    }
    if let Some(performed_at) = data.performed_at {
        experiment_model.performed_at = Set(Some(performed_at));
    }
    if let Some(temperature_ramp) = data.temperature_ramp {
        experiment_model.temperature_ramp = Set(Some(temperature_ramp));
    }
    if let Some(temperature_start) = data.temperature_start {
        experiment_model.temperature_start = Set(Some(temperature_start));
    }
    if let Some(temperature_end) = data.temperature_end {
        experiment_model.temperature_end = Set(Some(temperature_end));
    }
    experiment_model.is_calibration = Set(data.is_calibration);
    if let Some(remarks) = data.remarks {
        experiment_model.remarks = Set(Some(remarks));
    }
    if let Some(tray_configuration_id) = data.tray_configuration_id {
        experiment_model.tray_configuration_id = Set(Some(tray_configuration_id));
    }

    let experiment = experiment_model.insert(&txn).await?;

    // Handle regions if provided
    if !regions_to_create.is_empty() {
        for region in regions_to_create {
            // Convert Region to ActiveModel for insertion
            let region_active = crate::routes::tray_configurations::regions::models::ActiveModel {
                id: Set(Uuid::new_v4()),
                experiment_id: Set(experiment.id),
                treatment_id: Set(region.treatment_id),
                name: Set(region.name),
                display_colour_hex: Set(region.display_colour_hex),
                tray_id: Set(region.tray_id),
                col_min: Set(region.col_min),
                row_min: Set(region.row_min),
                col_max: Set(region.col_max),
                row_max: Set(region.row_max),
                dilution_factor: Set(region.dilution_factor),
                is_background_key: Set(region.is_background_key),
                created_at: Set(chrono::Utc::now()),
                last_updated: Set(chrono::Utc::now()),
            };

            region_active.insert(&txn).await?;
        }
    }

    txn.commit().await?;

    // Return basic experiment (bypass complex get_one_experiment for now)
    Ok(experiment.into())
}

pub(super) async fn update_experiment(
    db: &DatabaseConnection,
    id: Uuid,
    update_data: ExperimentUpdate,
) -> Result<Experiment, DbErr> {
    let txn = db.begin().await?;

    let existing: ActiveModel = Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or(DbErr::RecordNotFound("Experiment not found".to_string()))?
        .into();
    let regions = update_data.regions.clone();
    let updated_model =
        <ExperimentUpdate as MergeIntoActiveModel<ActiveModel>>::merge_into_activemodel(
            update_data,
            existing,
        )?;
    let _updated = updated_model.update(&txn).await?;

    // Handle regions update - delete existing regions and create new ones
    if !regions.is_empty() {
        // Delete existing regions for this experiment
        crate::routes::tray_configurations::regions::models::Entity::delete_many()
            .filter(
                crate::routes::tray_configurations::regions::models::Column::ExperimentId.eq(id),
            )
            .exec(&txn)
            .await?;

        // Create new regions
        for region in regions {
            // Convert Region to ActiveModel for insertion
            let region_active = crate::routes::tray_configurations::regions::models::ActiveModel {
                id: Set(Uuid::new_v4()),
                experiment_id: Set(id),
                treatment_id: Set(region.treatment_id.flatten()),
                name: Set(region.name.flatten()),
                display_colour_hex: Set(region.display_colour_hex.flatten()),
                tray_id: Set(region.tray_id.flatten()),
                col_min: Set(region.col_min.flatten()),
                row_min: Set(region.row_min.flatten()),
                col_max: Set(region.col_max.flatten()),
                row_max: Set(region.row_max.flatten()),
                dilution_factor: Set(region.dilution_factor.flatten()),
                is_background_key: Set(region.is_background_key.flatten().unwrap_or_default()),
                created_at: Set(chrono::Utc::now()),
                last_updated: Set(chrono::Utc::now()),
            };

            region_active.insert(&txn).await?;
        }
    }

    txn.commit().await?;

    // Return the complete experiment with regions
    get_one_experiment(db, id).await
}

pub(super) async fn get_all_experiments(
    db: &DatabaseConnection,
    condition: &Condition,
    order_column: Column,
    order_direction: Order,
    offset: u64,
    limit: u64,
) -> Result<Vec<ExperimentList>, DbErr> {
    let models = Entity::find()
        .filter(condition.clone())
        .order_by(order_column, order_direction)
        .offset(offset)
        .limit(limit)
        .all(db)
        .await?;

    let mut experiments = Vec::new();

    for model in models {
        let regions: Vec<crate::routes::tray_configurations::regions::models::Region> = model
            .find_related(crate::routes::tray_configurations::regions::models::Entity)
            .all(db)
            .await?
            .into_iter()
            .map(Into::into) // Direct conversion using EntityToModels
            .collect();

        let mut experiment: Experiment = model.into();
        experiment.regions = regions;

        experiments.push(experiment);
    }

    // Convert to ExperimentList
    Ok(experiments
        .into_iter()
        .map(std::convert::Into::into)
        .collect())
}
