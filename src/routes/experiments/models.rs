use super::services::{
    build_results_summary, create_region_active_models, region_model_to_input_with_treatment,
};
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels, traits::MergeIntoActiveModel};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveModelTrait, Condition, Order, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
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
    #[crudcrate(sortable, filterable)]
    pub temperature_ramp: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub temperature_start: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub temperature_end: Option<Decimal>,
    #[crudcrate(filterable)]
    pub is_calibration: bool,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub remarks: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub tray_configuration_id: Option<Uuid>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub last_updated: DateTime<Utc>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![])]
    pub assets: Vec<crate::routes::assets::models::Asset>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![])]
    pub regions: Vec<super::models_old::RegionInput>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = None)]
    pub results_summary: Option<super::models_old::ExperimentResultsSummary>,
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

// Custom crudcrate functions
async fn get_one_experiment(db: &DatabaseConnection, id: Uuid) -> Result<Experiment, DbErr> {
    let model = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound(format!("Experiment not found")))?;

    let s3_assets = model
        .find_related(crate::routes::assets::models::Entity)
        .all(db)
        .await?;

    let regions = model
        .find_related(crate::routes::tray_configurations::regions::models::Entity)
        .all(db)
        .await?;

    let mut regions_with_treatment = Vec::new();
    for region in regions {
        let region_input = region_model_to_input_with_treatment(region, db).await?;
        regions_with_treatment.push(region_input);
    }

    // Build results summary
    let results_summary = build_results_summary(id, db).await?;

    let mut experiment: Experiment = model.into();
    experiment.assets = s3_assets.into_iter().map(Into::into).collect();
    experiment.regions = regions_with_treatment;
    experiment.results_summary = results_summary;

    Ok(experiment)
}

async fn create_experiment(
    db: &DatabaseConnection,
    data: ExperimentCreate,
) -> Result<Experiment, DbErr> {
    let txn = db.begin().await?;

    // Store regions before conversion since they're not part of the DB model
    let regions_to_create = data.regions.clone();

    // Create the experiment first
    let experiment_model: ActiveModel = data.into();
    let experiment = experiment_model.insert(&txn).await?;

    // Handle regions if provided
    if !regions_to_create.is_empty() {
        let region_models = create_region_active_models(experiment.id, regions_to_create, &txn);

        for region_model in region_models {
            region_model.insert(&txn).await?;
        }
    }

    txn.commit().await?;

    // Return the complete experiment with regions
    get_one_experiment(db, experiment.id).await
}

async fn update_experiment(
    db: &DatabaseConnection,
    id: Uuid,
    update_data: ExperimentUpdate,
) -> Result<Experiment, DbErr> {
    let txn = db.begin().await?;

    let existing: ActiveModel = Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or(DbErr::RecordNotFound(format!("Experiment not found")))?
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
        let region_models = create_region_active_models(id, regions, &txn);

        for region_model in region_models {
            region_model.insert(&txn).await?;
        }
    }

    txn.commit().await?;

    // Return the complete experiment with regions
    get_one_experiment(db, id).await
}

async fn get_all_experiments(
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
        let s3_assets = model
            .find_related(crate::routes::assets::models::Entity)
            .all(db)
            .await?;

        let regions = model
            .find_related(crate::routes::tray_configurations::regions::models::Entity)
            .all(db)
            .await?;

        let mut regions_with_treatment = Vec::new();
        for region in regions {
            let region_input = region_model_to_input_with_treatment(region, db).await?;
            regions_with_treatment.push(region_input);
        }

        let mut experiment: Experiment = model.into();
        experiment.assets = s3_assets.into_iter().map(Into::into).collect();
        experiment.regions = regions_with_treatment;

        experiments.push(experiment);
    }

    // Convert to ExperimentList
    Ok(experiments.into_iter().map(|e| e.into()).collect())
}
