use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels};
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder, QuerySelect, entity::prelude::*};
use uuid::Uuid;

#[derive(Clone, Debug, ToSchema, PartialEq, Eq, Serialize, Deserialize)]
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
#[sea_orm(table_name = "treatments")]
#[crudcrate(
    generate_router,
    api_struct = "Treatment",
    name_singular = "treatment",
    name_plural = "treatments",
    description = "Treatments are applied to samples during experiments to study their effects on ice nucleation.",
    // fn_get_one = get_one_treatment,
    // fn_get_all = get_all_treatments,
)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[crudcrate(sortable, filterable)]
    pub name: TreatmentName,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub notes: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub sample_id: Option<Uuid>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub last_updated: DateTime<Utc>,
    #[sea_orm(column_type = "Decimal(Some((16, 10)))", nullable)]
    #[crudcrate(sortable, filterable)]
    pub enzyme_volume_litres: Option<Decimal>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], list_model = false)]
    pub experimental_results: Vec<ExperimentalResult>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::routes::trays::regions::models::Entity")]
    Regions,
    #[sea_orm(
        belongs_to = "crate::routes::samples::models::Entity",
        from = "Column::SampleId",
        to = "crate::routes::samples::models::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Samples,
}

impl Related<crate::routes::trays::regions::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Regions.def()
    }
}

impl Related<crate::routes::samples::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Samples.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, ToSchema, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "treatment_name")]
pub enum TreatmentName {
    #[sea_orm(string_value = "none")]
    #[serde(rename = "none")]
    None,
    #[sea_orm(string_value = "heat")]
    #[serde(rename = "heat")]
    Heat,
    #[sea_orm(string_value = "h2o2")]
    #[serde(rename = "h2o2")]
    H2o2,
}

// Helper function to format well coordinate (reused from samples)
fn format_well_coordinate_treatment(well: &crate::routes::trays::wells::models::Model) -> String {
    format!(
        "{}{}",
        char::from(b'A' + u8::try_from(well.column_number - 1).unwrap_or(0)),
        well.row_number
    )
}

async fn fetch_experimental_results_for_treatment(
    db: &DatabaseConnection,
    treatment_id: Uuid,
) -> Result<Vec<ExperimentalResult>, DbErr> {
    // Find all regions that use this treatment
    let regions = crate::routes::trays::regions::models::Entity::find()
        .filter(crate::routes::trays::regions::models::Column::TreatmentId.eq(treatment_id))
        .find_with_related(crate::routes::experiments::models::Entity)
        .all(db)
        .await?;

    let mut experimental_results = Vec::new();

    for (region, experiments) in regions {
        for experiment in experiments {
            // Find wells that fall within this region's coordinates
            let wells = if let (Some(row_min), Some(row_max), Some(col_min), Some(col_max)) = (
                region.row_min,
                region.row_max,
                region.col_min,
                region.col_max,
            ) {
                crate::routes::trays::wells::models::Entity::find()
                    .filter(
                        crate::routes::trays::wells::models::Column::RowNumber
                            .gte(row_min + 1) // Convert 0-based to 1-based
                            .and(
                                crate::routes::trays::wells::models::Column::RowNumber
                                    .lte(row_max + 1),
                            )
                            .and(
                                crate::routes::trays::wells::models::Column::ColumnNumber
                                    .gte(col_min + 1),
                            )
                            .and(
                                crate::routes::trays::wells::models::Column::ColumnNumber
                                    .lte(col_max + 1),
                            ),
                    )
                    .all(db)
                    .await?
            } else {
                vec![]
            };

            for well in wells {
                let well_coordinate = format_well_coordinate_treatment(&well);

                // Get tray name
                let tray = crate::routes::trays::models::Entity::find_by_id(well.tray_id)
                    .one(db)
                    .await?;

                // For now, simplified experimental result - you may want to add freezing metrics
                experimental_results.push(ExperimentalResult {
                    experiment_id: experiment.id,
                    experiment_name: experiment.name.clone(),
                    experiment_date: experiment.performed_at.map(|dt| dt.with_timezone(&Utc)),
                    well_coordinate,
                    tray_name: tray.and_then(|t| t.name),
                    freezing_temperature_avg: None, // TODO: Implement freezing metrics
                    freezing_time_seconds: None,    // TODO: Implement freezing metrics
                    treatment_name: Some(format!("{:?}", region.treatment_id)),
                    treatment_id: Some(treatment_id),
                    dilution_factor: region.dilution_factor,
                    final_state: "unknown".to_string(), // TODO: Implement final state
                });
            }
        }
    }

    Ok(experimental_results)
}

// Custom crudcrate functions
async fn get_one_treatment(db: &DatabaseConnection, id: Uuid) -> Result<Treatment, DbErr> {
    let model = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Treatment not found".to_string()))?;

    let experimental_results = fetch_experimental_results_for_treatment(db, id).await?;

    let mut treatment: Treatment = model.into();
    treatment.experimental_results = experimental_results;

    Ok(treatment)
}

async fn get_all_treatments(
    db: &DatabaseConnection,
    condition: sea_orm::Condition,
    order_column: Column,
    order_direction: sea_orm::Order,
    offset: u64,
    limit: u64,
) -> Result<Vec<Treatment>, DbErr> {
    let models = Entity::find()
        .filter(condition)
        .order_by(order_column, order_direction)
        .offset(offset)
        .limit(limit)
        .all(db)
        .await?;

    let mut treatments: Vec<Treatment> = models.into_iter().map(Treatment::from).collect();

    for treatment in treatments.iter_mut() {
        treatment.experimental_results =
            fetch_experimental_results_for_treatment(db, treatment.id).await?;
    }

    Ok(treatments)
}
