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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SampleTreatment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    pub name: crate::routes::treatments::models::TreatmentName,
    pub notes: Option<String>,
    pub enzyme_volume_litres: Option<Decimal>,
}

impl From<crate::routes::treatments::models::Model> for SampleTreatment {
    fn from(model: crate::routes::treatments::models::Model) -> Self {
        Self {
            id: Some(model.id),
            name: model.name,
            notes: model.notes,
            enzyme_volume_litres: model.enzyme_volume_litres,
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct SampleTreatmentCreate {
    pub name: crate::routes::treatments::models::TreatmentName,
    pub notes: Option<String>,
    pub enzyme_volume_litres: Option<Decimal>,
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
    fn_create = create_sample,
    fn_update = update_sample,
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
    #[crudcrate(non_db_attr = true, default = vec![], list_model = false)]
    pub treatments: Vec<SampleTreatment>,
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

// Custom create structure to handle treatments
#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct SampleCreateCustom {
    pub name: String,
    pub r#type: SampleType,
    pub material_description: Option<String>,
    pub extraction_procedure: Option<String>,
    pub filter_substrate: Option<String>,
    pub suspension_volume_litres: Option<Decimal>,
    pub air_volume_litres: Option<Decimal>,
    pub water_volume_litres: Option<Decimal>,
    pub initial_concentration_gram_l: Option<Decimal>,
    pub well_volume_litres: Option<Decimal>,
    pub remarks: Option<String>,
    pub location_id: Option<Uuid>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: Option<DateTime<Utc>>,
    pub flow_litres_per_minute: Option<Decimal>,
    pub total_volume: Option<Decimal>,
    #[serde(default)]
    pub treatments: Vec<SampleTreatmentCreate>,
}

// Helper function to fetch wells within region coordinates
async fn fetch_wells_in_region(
    db: &DatabaseConnection,
    region: &crate::routes::trays::regions::models::Model,
) -> Result<Vec<crate::routes::trays::wells::models::Model>, DbErr> {
    if let (Some(row_min), Some(row_max), Some(col_min), Some(col_max)) = (
        region.row_min,
        region.row_max,
        region.col_min,
        region.col_max,
    ) {
        crate::routes::trays::wells::models::Entity::find()
            .filter(
                crate::routes::trays::wells::models::Column::RowNumber
                    .gte(row_min + 1) // Convert 0-based to 1-based
                    .and(crate::routes::trays::wells::models::Column::RowNumber.lte(row_max + 1))
                    .and(crate::routes::trays::wells::models::Column::ColumnNumber.gte(col_min + 1))
                    .and(
                        crate::routes::trays::wells::models::Column::ColumnNumber.lte(col_max + 1),
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
fn format_well_coordinate(well: &crate::routes::trays::wells::models::Model) -> String {
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
    let regions = crate::routes::trays::regions::models::Entity::find()
        .filter(
            crate::routes::trays::regions::models::Column::TreatmentId.is_in(treatment_ids.clone()),
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

                // Get tray name
                let tray = crate::routes::trays::models::Entity::find_by_id(well.tray_id)
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

// Custom crudcrate functions
async fn get_one_sample(db: &DatabaseConnection, id: Uuid) -> Result<Sample, DbErr> {
    let model = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Sample not found".to_string()))?;

    let treatments = model
        .find_related(crate::routes::treatments::models::Entity)
        .all(db)
        .await?;

    let experimental_results = fetch_experimental_results_for_sample(db, id).await?;

    let mut sample: Sample = model.into();
    sample.treatments = treatments.into_iter().map(SampleTreatment::from).collect();
    sample.experimental_results = experimental_results;

    Ok(sample)
}

async fn get_all_samples(
    db: &DatabaseConnection,
    condition: &Condition,
    order_column: Column,
    order_direction: Order,
    offset: u64,
    limit: u64,
) -> Result<Vec<Sample>, DbErr> {
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
            .map(SampleTreatment::from)
            .collect();

        // Fetch experimental results for each sample
        sample.experimental_results = fetch_experimental_results_for_sample(db, sample.id).await?;
    }

    Ok(samples)
}

async fn create_sample(
    db: &DatabaseConnection,
    create_data: SampleCreate,
) -> Result<Sample, DbErr> {
    // Extract treatments if present
    let treatments = if create_data.treatments.is_empty() {
        None
    } else {
        Some(create_data.treatments.clone())
    };

    let active_model = ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        name: ActiveValue::Set(create_data.name),
        r#type: ActiveValue::Set(create_data.r#type),
        material_description: ActiveValue::Set(create_data.material_description),
        extraction_procedure: ActiveValue::Set(create_data.extraction_procedure),
        filter_substrate: ActiveValue::Set(create_data.filter_substrate),
        suspension_volume_litres: ActiveValue::Set(create_data.suspension_volume_litres),
        air_volume_litres: ActiveValue::Set(create_data.air_volume_litres),
        water_volume_litres: ActiveValue::Set(create_data.water_volume_litres),
        initial_concentration_gram_l: ActiveValue::Set(create_data.initial_concentration_gram_l),
        well_volume_litres: ActiveValue::Set(create_data.well_volume_litres),
        remarks: ActiveValue::Set(create_data.remarks),
        created_at: ActiveValue::Set(chrono::Utc::now().into()),
        last_updated: ActiveValue::Set(chrono::Utc::now().into()),
        location_id: ActiveValue::Set(create_data.location_id),
        latitude: ActiveValue::Set(create_data.latitude),
        longitude: ActiveValue::Set(create_data.longitude),
        start_time: ActiveValue::Set(create_data.start_time.map(std::convert::Into::into)),
        stop_time: ActiveValue::Set(create_data.stop_time.map(std::convert::Into::into)),
        flow_litres_per_minute: ActiveValue::Set(create_data.flow_litres_per_minute),
        total_volume: ActiveValue::Set(create_data.total_volume),
    };

    let inserted = active_model.insert(db).await?;
    let sample_id = inserted.id;

    // Insert treatments if provided
    if let Some(treatments) = treatments {
        for treatment in treatments {
            let treatment_id = treatment.id.unwrap_or_else(Uuid::new_v4);
            let active_treatment = crate::routes::treatments::models::ActiveModel {
                id: ActiveValue::Set(treatment_id),
                sample_id: ActiveValue::Set(Some(sample_id)),
                name: ActiveValue::Set(treatment.name),
                notes: ActiveValue::Set(treatment.notes),
                enzyme_volume_litres: ActiveValue::Set(treatment.enzyme_volume_litres),
                ..Default::default()
            };
            let _ = active_treatment.insert(db).await?;
        }
    }

    // Reload with treatments
    get_one_sample(db, sample_id).await
}

async fn update_sample(
    db: &DatabaseConnection,
    id: Uuid,
    update_data: SampleUpdate,
) -> Result<Sample, DbErr> {
    // Note: This is a simplified version. The full implementation would handle
    // treatment updates similar to the old models_old.rs implementation
    let existing = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Sample not found".to_string()))?;

    let mut active_model: ActiveModel = existing.into();

    // Update fields from update_data
    // This would need proper implementation based on the SampleUpdate struct

    let updated = active_model.update(db).await?;
    get_one_sample(db, updated.id).await
}
