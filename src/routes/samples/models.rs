use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, Condition, DatabaseConnection, EntityTrait, Order, QueryOrder, QuerySelect,
    entity::prelude::*,
};
use uuid::Uuid;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, ToSchema, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "sample_type")]
#[serde(rename_all = "snake_case")]
pub enum SampleType {
    #[sea_orm(string_value = "bulk")]
    Bulk,
    #[sea_orm(string_value = "filter")]
    Filter,
    #[sea_orm(string_value = "procedural_blank")]
    ProceduralBlank,
    #[sea_orm(string_value = "pure_water")]
    PureWater,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ExperimentalResult {
    pub experiment_id: Uuid,
    pub experiment_name: String,
    pub experiment_date: Option<DateTime<Utc>>,
    pub well_coordinate: String,
    pub tray_name: Option<String>,
    pub freezing_temperature_avg: Option<Decimal>,
    pub freezing_time_seconds: Option<i64>,
    pub treatment_name: Option<String>,
    pub treatment_id: Option<Uuid>,
    pub dilution_factor: Option<i32>,
    pub final_state: String,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, EntityToModels)]
#[sea_orm(table_name = "samples")]
#[crudcrate(
    generate_router,
    api_struct = "Sample",
    name_singular = "sample",
    name_plural = "samples",
    description = "This resource manages samples associated with experiments.",
    fn_get_one = get_one_sample,
    fn_create = create_sample_with_treatments,
    fn_update = update_sample_with_treatments,
    fn_get_all = get_all_samples,
)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[sea_orm(column_type = "Text")]
    #[crudcrate(sortable, filterable, fulltext)]
    pub name: String,
    #[crudcrate(sortable, filterable)]
    pub r#type: SampleType,
    #[crudcrate(sortable)]
    pub start_time: Option<DateTime<Utc>>,
    #[crudcrate(sortable)]
    pub stop_time: Option<DateTime<Utc>>,
    #[sea_orm(column_type = "Decimal(Some((16, 10)))", nullable)]
    #[crudcrate(sortable, filterable)]
    pub flow_litres_per_minute: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((16, 10)))", nullable)]
    #[crudcrate(sortable, filterable)]
    pub total_volume: Option<Decimal>,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub material_description: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub extraction_procedure: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub filter_substrate: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub suspension_volume_litres: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub air_volume_litres: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub water_volume_litres: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub initial_concentration_gram_l: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub well_volume_litres: Option<Decimal>,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub remarks: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((9, 6)))", nullable)]
    #[crudcrate(sortable)]
    pub longitude: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((9, 6)))", nullable)]
    #[crudcrate(sortable)]
    pub latitude: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub location_id: Option<Uuid>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub last_updated: DateTime<Utc>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], use_target_models)]
    pub treatments: Vec<crate::routes::treatments::models::Treatment>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], list_model = false)]
    pub experimental_results: Vec<ExperimentalResult>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::routes::locations::models::Entity",
        from = "Column::LocationId",
        to = "crate::routes::locations::models::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Locations,
    #[sea_orm(has_many = "crate::routes::treatments::models::Entity")]
    Treatments,
}

impl Related<crate::routes::locations::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Locations.def()
    }
}

impl Related<crate::routes::treatments::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Treatments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// Helper function to fetch wells within region coordinates
async fn fetch_wells_in_region(
    db: &DatabaseConnection,
    region: &crate::routes::tray_configurations::regions::models::Model,
) -> Result<Vec<crate::routes::tray_configurations::wells::models::Model>, DbErr> {
    if let (Some(row_min), Some(row_max), Some(col_min), Some(col_max)) = (
        region.row_min,
        region.row_max,
        region.col_min,
        region.col_max,
    ) {
        crate::routes::tray_configurations::wells::models::Entity::find()
            .filter(
                crate::routes::tray_configurations::wells::models::Column::RowNumber
                    .gte(row_min + 1) // Convert 0-based to 1-based
                    .and(
                        crate::routes::tray_configurations::wells::models::Column::RowNumber
                            .lte(row_max + 1),
                    )
                    .and(
                        crate::routes::tray_configurations::wells::models::Column::ColumnNumber
                            .gte(col_min + 1),
                    )
                    .and(
                        crate::routes::tray_configurations::wells::models::Column::ColumnNumber
                            .lte(col_max + 1),
                    ),
            )
            .all(db)
            .await
    } else {
        Ok(vec![])
    }
}

// Helper function to determine final state of well
async fn determine_final_state(
    db: &DatabaseConnection,
    well_id: Uuid,
    experiment_id: Uuid,
) -> Result<String, DbErr> {
    let has_frozen_transition =
        crate::routes::experiments::phase_transitions::models::Entity::find()
            .filter(
                crate::routes::experiments::phase_transitions::models::Column::WellId
                    .eq(well_id)
                    .and(
                        crate::routes::experiments::phase_transitions::models::Column::ExperimentId
                            .eq(experiment_id),
                    )
                    .and(
                        crate::routes::experiments::phase_transitions::models::Column::NewState
                            .eq(1),
                    ),
            )
            .one(db)
            .await?
            .is_some();

    Ok(if has_frozen_transition {
        "frozen".to_string()
    } else {
        "liquid".to_string()
    })
}

// Helper function to format well coordinate
fn format_well_coordinate(
    well: &crate::routes::tray_configurations::wells::models::Model,
) -> String {
    format!(
        "{}{}",
        char::from(b'A' + u8::try_from(well.column_number - 1).unwrap_or(0)),
        well.row_number
    )
}

// Helper function to calculate freezing time and temperature
async fn calculate_freezing_metrics(
    db: &DatabaseConnection,
    well_id: Uuid,
    experiment: &crate::routes::experiments::models::Model,
) -> Result<(Option<i64>, Option<Decimal>), DbErr> {
    let phase_transitions = crate::routes::experiments::phase_transitions::models::Entity::find()
        .filter(
            crate::routes::experiments::phase_transitions::models::Column::WellId
                .eq(well_id)
                .and(
                    crate::routes::experiments::phase_transitions::models::Column::ExperimentId
                        .eq(experiment.id),
                )
                .and(
                    crate::routes::experiments::phase_transitions::models::Column::PreviousState
                        .eq(0),
                )
                .and(crate::routes::experiments::phase_transitions::models::Column::NewState.eq(1)),
        )
        .find_with_related(crate::routes::experiments::temperatures::models::Entity)
        .all(db)
        .await?;

    if let Some((_transition, temp_readings)) = phase_transitions.first() {
        let freezing_time = if let Some(temp_reading) = temp_readings.first() {
            if let Some(experiment_start) = experiment.performed_at {
                let transition_time = temp_reading.timestamp;
                Some((transition_time - experiment_start).num_seconds())
            } else {
                None
            }
        } else {
            None
        };

        let avg_temp = if let Some(temp_reading) = temp_readings.first() {
            // Calculate average of all 8 temperature probes
            let temps = vec![
                temp_reading.probe_1,
                temp_reading.probe_2,
                temp_reading.probe_3,
                temp_reading.probe_4,
                temp_reading.probe_5,
                temp_reading.probe_6,
                temp_reading.probe_7,
                temp_reading.probe_8,
            ];
            let valid_temps: Vec<Decimal> = temps.into_iter().flatten().collect();
            if valid_temps.is_empty() {
                None
            } else {
                Some(valid_temps.iter().sum::<Decimal>() / Decimal::from(valid_temps.len()))
            }
        } else {
            None
        };

        Ok((freezing_time, avg_temp))
    } else {
        Ok((None, None))
    }
}

async fn fetch_experimental_results_for_sample(
    db: &DatabaseConnection,
    sample_id: Uuid,
) -> Result<Vec<ExperimentalResult>, DbErr> {
    // Find all treatments that use this sample
    let treatments = crate::routes::treatments::models::Entity::find()
        .filter(crate::routes::treatments::models::Column::SampleId.eq(sample_id))
        .all(db)
        .await?;

    if treatments.is_empty() {
        return Ok(vec![]);
    }

    let treatment_ids: Vec<Uuid> = treatments.iter().map(|t| t.id).collect();

    // Find all regions that use these treatments
    let regions = crate::routes::tray_configurations::regions::models::Entity::find()
        .filter(
            crate::routes::tray_configurations::regions::models::Column::TreatmentId
                .is_in(treatment_ids.clone()),
        )
        .find_with_related(crate::routes::experiments::models::Entity)
        .all(db)
        .await?;

    let mut experimental_results = Vec::new();

    // Iterate through each region and its associated experiments
    for (region, experiments) in regions {
        for experiment in experiments {
            let wells = fetch_wells_in_region(db, &region).await?;

            for well in wells {
                let (freezing_time_seconds, freezing_temperature_avg) =
                    calculate_freezing_metrics(db, well.id, &experiment).await?;

                let final_state = determine_final_state(db, well.id, experiment.id).await?;

                let well_coordinate = format_well_coordinate(&well);

                // Find the treatment for this region
                let treatment = treatments
                    .iter()
                    .find(|t| t.id == region.treatment_id.unwrap_or_default());

                // Get tray name (from configuration assignments with embedded tray data)
                let tray = crate::routes::tray_configurations::trays::models::Entity::find_by_id(
                    well.tray_id,
                )
                .one(db)
                .await?;

                experimental_results.push(ExperimentalResult {
                    experiment_id: experiment.id,
                    experiment_name: experiment.name.clone(),
                    experiment_date: experiment.performed_at.map(|dt| dt.with_timezone(&Utc)),
                    well_coordinate,
                    tray_name: tray.and_then(|t| t.name),
                    freezing_temperature_avg,
                    freezing_time_seconds,
                    treatment_name: treatment.map(|t| format!("{:?}", t.name)),
                    treatment_id: treatment.map(|t| t.id),
                    dilution_factor: region.dilution_factor,
                    final_state,
                });
            }
        }
    }

    Ok(experimental_results)
}

// Custom functions that handle treatments while leveraging crudcrate for base operations

// Simple get_one that loads treatments
async fn get_one_sample(db: &DatabaseConnection, id: Uuid) -> Result<Sample, DbErr> {
    let model = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Sample not found".to_string()))?;

    let treatments = model
        .find_related(crate::routes::treatments::models::Entity)
        .all(db)
        .await?;

    let mut sample: Sample = model.into();
    sample.treatments = treatments
        .into_iter()
        .map(|t| crate::routes::treatments::models::Treatment::from(t))
        .collect();

    // Note: experimental_results can be loaded on-demand if needed

    Ok(sample)
}

// Simple get_all that loads treatments
async fn get_all_samples(
    db: &DatabaseConnection,
    condition: &sea_orm::Condition,
    order_column: Column,
    order_direction: sea_orm::Order,
    offset: u64,
    limit: u64,
) -> Result<Vec<SampleList>, DbErr> {
    let models = Entity::find()
        .filter(condition.clone())
        .order_by(order_column, order_direction)
        .offset(offset)
        .limit(limit)
        .all(db)
        .await?;

    let treatments_vec = models
        .load_many(crate::routes::treatments::models::Entity, db)
        .await?;

    let mut samples: Vec<Sample> = models.into_iter().map(Sample::from).collect();
    for (i, sample) in samples.iter_mut().enumerate() {
        sample.treatments = treatments_vec[i]
            .iter()
            .cloned()
            .map(crate::routes::treatments::models::Treatment::from)
            .collect();
    }

    // Convert to SampleList for response
    Ok(samples.into_iter().map(SampleList::from).collect())
}

async fn create_sample_with_treatments(
    db: &DatabaseConnection,
    create_data: SampleCreate,
) -> Result<Sample, DbErr> {
    // Extract treatments before creating sample
    let treatments_to_create = if create_data.treatments.is_empty() {
        None
    } else {
        Some(create_data.treatments.clone())
    };

    // Use the auto-generated default create logic by creating ActiveModel directly
    let active_model: ActiveModel = create_data.into();
    let inserted = active_model.insert(db).await?;
    let sample_id = inserted.id;

    // Create treatments using CRUDResource methods
    if let Some(treatments) = treatments_to_create {
        for treatment_create in treatments {
            let mut treatment_with_sample = treatment_create;
            treatment_with_sample.sample_id = Some(sample_id);
            let _ = crate::routes::treatments::models::Treatment::create(db, treatment_with_sample)
                .await?;
        }
    }

    // Return the created sample with treatments loaded
    Sample::get_one(db, sample_id).await
}

async fn update_sample_with_treatments(
    db: &DatabaseConnection,
    id: Uuid,
    update_data: SampleUpdate,
) -> Result<Sample, DbErr> {
    // Extract treatments before updating sample
    let treatments_to_recreate = if update_data.treatments.is_empty() {
        None
    } else {
        Some(update_data.treatments.clone())
    };

    // Use the auto-generated default update logic
    let sample = Sample::update(db, id, update_data).await?;

    // Handle treatments if provided (delete and recreate approach)
    if let Some(treatments) = treatments_to_recreate {
        // Delete existing treatments
        let _ = crate::routes::treatments::models::Entity::delete_many()
            .filter(crate::routes::treatments::models::Column::SampleId.eq(id))
            .exec(db)
            .await?;

        // Create new treatments
        for treatment_update in treatments {
            let treatment_create = crate::routes::treatments::models::TreatmentCreate {
                name: treatment_update
                    .name
                    .unwrap_or_default()
                    .unwrap_or(crate::routes::treatments::models::TreatmentName::None),
                notes: treatment_update.notes.unwrap_or_default(),
                enzyme_volume_litres: treatment_update.enzyme_volume_litres.unwrap_or_default(),
                sample_id: Some(id),
                experimental_results: vec![],
            };
            let _ =
                crate::routes::treatments::models::Treatment::create(db, treatment_create).await?;
        }
    }

    // Return the updated sample with treatments loaded
    Sample::get_one(db, id).await
}
