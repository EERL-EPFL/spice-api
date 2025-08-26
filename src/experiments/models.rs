use crate::experiments::services::build_tray_centric_results;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels, traits::MergeIntoActiveModel};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveValue::Set, Condition, EntityTrait, Order, QueryOrder, QuerySelect, TransactionTrait,
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
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable)]
    pub last_updated: DateTime<Utc>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], list_model=false, use_target_models)]
    pub regions: Vec<crate::tray_configurations::regions::models::Region>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = None, list_model=false)]
    pub results: Option<super::models::ExperimentResultsResponse>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::tray_configurations::regions::models::Entity")]
    Regions,
    #[sea_orm(has_many = "crate::assets::models::Entity")]
    S3Assets,
    #[sea_orm(has_many = "crate::experiments::temperatures::models::Entity")]
    TemperatureReadings,
    #[sea_orm(
        belongs_to = "crate::tray_configurations::models::Entity",
        from = "Column::TrayConfigurationId",
        to = "crate::tray_configurations::models::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    TrayConfigurations,
    #[sea_orm(has_many = "crate::experiments::phase_transitions::models::Entity")]
    WellPhaseTransitions,
}

impl Related<crate::tray_configurations::regions::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Regions.def()
    }
}

impl Related<crate::assets::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::S3Assets.def()
    }
}

impl Related<crate::experiments::temperatures::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TemperatureReadings.def()
    }
}

impl Related<crate::tray_configurations::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TrayConfigurations.def()
    }
}

impl Related<crate::experiments::phase_transitions::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WellPhaseTransitions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProbeTemperatureReadingWithMetadata {
    pub id: Uuid,
    pub temperature_reading_id: Uuid,
    pub temperature: rust_decimal::Decimal,
    pub created_at: DateTime<Utc>,
    // Probe metadata
    pub probe_id: Uuid,
    pub probe_name: String,
    pub probe_data_column_index: i32,
    pub probe_position_x: rust_decimal::Decimal,
    pub probe_position_y: rust_decimal::Decimal,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TemperatureDataWithProbes {
    // Flattened temperature reading fields
    pub id: Uuid,
    pub experiment_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub image_filename: Option<String>,
    pub average: Option<rust_decimal::Decimal>,
    // All probe readings for this timestamp with metadata
    pub probe_readings: Vec<ProbeTemperatureReadingWithMetadata>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TrayWellSummary {
    pub row_letter: String,
    pub column_number: i32,
    pub coordinate: String, // e.g., "A1", "B2"
    pub sample: Option<crate::samples::models::Sample>,
    pub treatment: Option<crate::treatments::models::Treatment>, // Full treatment object with enzyme volume
    pub dilution_factor: Option<i32>,
    pub first_phase_change_time: Option<DateTime<Utc>>,
    pub temperatures: Option<TemperatureDataWithProbes>,
    pub total_phase_changes: usize,
    pub image_asset_id: Option<Uuid>, // Asset ID for the image at freeze time
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TrayResultsSummary {
    pub tray_id: String,
    pub tray_name: Option<String>,
    pub wells: Vec<TrayWellSummary>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ExperimentResultsSummaryCompact {
    pub total_time_points: usize,
    pub first_timestamp: Option<DateTime<Utc>>,
    pub last_timestamp: Option<DateTime<Utc>>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ExperimentResultsResponse {
    pub summary: ExperimentResultsSummaryCompact,
    pub trays: Vec<TrayResultsSummary>,
}

// Helper function to enhance regions with treatment and sample data
async fn enhance_regions_with_treatment_data(
    region_models: Vec<crate::tray_configurations::regions::models::Model>,
    db: &DatabaseConnection,
) -> Result<Vec<crate::tray_configurations::regions::models::Region>, DbErr> {
    // Extract all unique treatment IDs from regions
    let treatment_ids: Vec<Uuid> = region_models
        .iter()
        .filter_map(|r| r.treatment_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // If no treatments to load, return regions as-is
    if treatment_ids.is_empty() {
        return Ok(region_models.into_iter().map(Into::into).collect());
    }

    // Load all treatments with their samples in one query
    let treatments_with_samples = crate::treatments::models::Entity::find()
        .filter(crate::treatments::models::Column::Id.is_in(treatment_ids))
        .find_with_related(crate::samples::models::Entity)
        .all(db)
        .await?;

    // Build a map of treatment_id -> (treatment, sample)
    let mut treatment_map: std::collections::HashMap<
        Uuid,
        (
            crate::treatments::models::Treatment,
            Option<crate::samples::models::Sample>,
        ),
    > = std::collections::HashMap::new();

    for (treatment_model, samples) in treatments_with_samples {
        let treatment: crate::treatments::models::Treatment = treatment_model.into();
        let sample = samples.into_iter().next().map(Into::into);
        treatment_map.insert(treatment.id, (treatment, sample));
    }

    // Convert region models to Region structs with embedded treatment data
    let enhanced_regions = region_models
        .into_iter()
        .map(|region_model| {
            // Get treatment data if available
            let treatment = if let Some(treatment_id) = region_model.treatment_id {
                treatment_map.get(&treatment_id).map(|(treatment, sample)| {
                    crate::tray_configurations::regions::models::RegionTreatmentSummary {
                        id: treatment.id,
                        name: match treatment.name {
                            crate::treatments::models::TreatmentName::None => "none".to_string(),
                            crate::treatments::models::TreatmentName::Heat => "heat".to_string(),
                            crate::treatments::models::TreatmentName::H2o2 => "h2o2".to_string(),
                        },
                        notes: treatment.notes.clone(),
                        enzyme_volume_litres: treatment.enzyme_volume_litres,
                        sample: sample.clone(),
                    }
                })
            } else {
                None
            };

            // Create Region with treatment data
            crate::tray_configurations::regions::models::Region {
                id: region_model.id,
                experiment_id: region_model.experiment_id,
                treatment_id: region_model.treatment_id,
                name: region_model.name,
                display_colour_hex: region_model.display_colour_hex,
                tray_id: region_model.tray_id,
                col_min: region_model.col_min,
                row_min: region_model.row_min,
                col_max: region_model.col_max,
                row_max: region_model.row_max,
                dilution_factor: region_model.dilution_factor,
                is_background_key: region_model.is_background_key,
                created_at: region_model.created_at,
                last_updated: region_model.last_updated,
                treatment,
            }
        })
        .collect();

    Ok(enhanced_regions)
}

pub(super) async fn get_one_experiment(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Experiment, DbErr> {
    let model = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("Experiment not found".to_string()))?;

    // Load regions with enhanced treatment and sample data
    let region_models = model
        .find_related(crate::tray_configurations::regions::models::Entity)
        .all(db)
        .await?;

    // Enhance regions with treatment and sample data
    let enhanced_regions = enhance_regions_with_treatment_data(region_models, db).await?;

    let mut experiment: Experiment = model.into();
    experiment.regions = enhanced_regions;
    experiment.results = build_tray_centric_results(id, db).await?;

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
            let region_active = crate::tray_configurations::regions::models::ActiveModel {
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
        crate::tray_configurations::regions::models::Entity::delete_many()
            .filter(crate::tray_configurations::regions::models::Column::ExperimentId.eq(id))
            .exec(&txn)
            .await?;

        // Create new regions
        for region in regions {
            // Convert Region to ActiveModel for insertion
            let region_active = crate::tray_configurations::regions::models::ActiveModel {
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

    let mut experiment_lists = Vec::new();

    for model in models {
        let regions: Vec<crate::tray_configurations::regions::models::Region> = model
            .find_related(crate::tray_configurations::regions::models::Entity)
            .all(db)
            .await?
            .into_iter()
            .map(Into::into) // Direct conversion using EntityToModels
            .collect();

        let mut experiment: Experiment = model.into();
        experiment.regions = regions;

        // Convert to ExperimentList
        let experiment_list: ExperimentList = experiment.into();

        experiment_lists.push(experiment_list);
    }

    Ok(experiment_lists)
}
