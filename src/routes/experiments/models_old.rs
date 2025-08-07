use super::services::{
    build_results_summary, create_region_active_models, region_model_to_input_with_treatment,
};
use crate::routes::experiments::models::Model;
use crate::routes::treatments::models::TreatmentName;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel, traits::MergeIntoActiveModel};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, Condition, DatabaseConnection, EntityTrait, Order, QueryOrder,
    QuerySelect, TransactionTrait, entity::prelude::*,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(ToSchema, Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
pub struct TreatmentInfo {
    pub id: Uuid,
    pub name: TreatmentName,
    pub notes: Option<String>,
    pub enzyme_volume_litres: Option<Decimal>,
    pub sample: Option<SampleInfo>,
}

#[derive(ToSchema, Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
pub struct SampleInfo {
    pub id: Uuid,
    pub name: String,
    pub location: Option<LocationInfo>,
}

#[derive(ToSchema, Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
pub struct LocationInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(ToSchema, Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
pub struct TemperatureProbeValues {
    pub probe_1: Option<Decimal>,
    pub probe_2: Option<Decimal>,
    pub probe_3: Option<Decimal>,
    pub probe_4: Option<Decimal>,
    pub probe_5: Option<Decimal>,
    pub probe_6: Option<Decimal>,
    pub probe_7: Option<Decimal>,
    pub probe_8: Option<Decimal>,
    pub average: Option<Decimal>,
}

#[derive(ToSchema, Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct WellSummary {
    pub row: i32,
    pub col: i32,
    pub coordinate: String, // e.g., "A1", "B2"
    pub first_phase_change_time: Option<DateTime<Utc>>,
    pub first_phase_change_seconds: Option<i64>, // seconds from experiment start
    pub first_phase_change_temperature_probes: Option<TemperatureProbeValues>, // Temperature probe values at first phase change
    pub final_state: Option<String>, // "frozen", "liquid", "no_data"
    // pub sample_name: Option<String>,
    // pub treatment_name: Option<String>,
    // pub treatment_id: Option<Uuid>,
    pub tray_id: Option<String>, // UUID of the tray
    pub tray_name: Option<String>,
    pub dilution_factor: Option<i32>,
    // Full objects with UUIDs for UI linking
    pub treatment: Option<crate::routes::treatments::models::Treatment>,
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
    // Keep original format for backwards compatibility
    pub well_summaries: Vec<WellSummary>,
}

#[derive(ToSchema, Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct TrayInfo {
    pub id: Uuid,
    pub name: Option<String>,
    pub sequence_id: i32,
    pub qty_x_axis: Option<i32>,
    pub qty_y_axis: Option<i32>,
    pub well_relative_diameter: Option<String>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RegionInput {
    pub name: Option<String>,
    pub tray_sequence_id: Option<i32>, // Renamed from tray_id for clarity
    pub col_min: Option<i32>,
    pub col_max: Option<i32>,
    pub row_min: Option<i32>,
    pub row_max: Option<i32>,
    pub color: Option<String>, // hex color
    pub dilution: Option<String>,
    pub treatment_id: Option<Uuid>,
    pub is_background_key: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub treatment: Option<crate::routes::treatments::models::Treatment>,
    pub sample: Option<crate::routes::samples::models::Sample>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tray: Option<TrayInfo>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
struct TrayRegions {
    pub id: Uuid,
    pub experiment_id: Uuid,
    pub treatment_id: Option<Uuid>,
    pub name: Option<String>,
    pub display_colour_hex: Option<String>,
    pub tray_id: Option<i32>,
    pub col_min: Option<i32>,
    pub row_min: Option<i32>,
    pub col_max: Option<i32>,
    pub row_max: Option<i32>,
    pub dilution_factor: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl From<crate::routes::trays::regions::models::Model> for TrayRegions {
    fn from(region: crate::routes::trays::regions::models::Model) -> Self {
        Self {
            id: region.id,
            experiment_id: region.experiment_id,
            treatment_id: region.treatment_id,
            name: region.name,
            display_colour_hex: region.display_colour_hex,
            tray_id: region.tray_id,
            col_min: region.col_min,
            row_min: region.row_min,
            col_max: region.col_max,
            row_max: region.row_max,
            dilution_factor: region.dilution_factor,
            created_at: region.created_at.into(),
            last_updated: region.last_updated.into(),
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "crate::routes::experiments::models::ActiveModel"]
pub struct Experiment {
    #[crudcrate(update_model = false, create_model = false, on_create = Uuid::new_v4())]
    id: Uuid,
    name: String,
    // sample_id: Uuid,
    tray_configuration_id: Option<Uuid>,
    username: Option<String>,
    performed_at: Option<DateTime<Utc>>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    last_updated: DateTime<Utc>,
    temperature_ramp: Option<Decimal>,
    temperature_start: Option<Decimal>,
    temperature_end: Option<Decimal>,
    is_calibration: bool,
    remarks: Option<String>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    assets: Vec<crate::routes::assets::models::Asset>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    regions: Vec<RegionInput>,
    #[crudcrate(non_db_attr = true, default = None)]
    results_summary: Option<ExperimentResultsSummary>,
}

impl From<Model> for Experiment {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            tray_configuration_id: model.tray_configuration_id,
            username: model.username,
            performed_at: model.performed_at.map(|dt| dt.with_timezone(&Utc)),
            created_at: model.created_at.into(),
            last_updated: model.last_updated.into(),
            temperature_ramp: model.temperature_ramp,
            temperature_start: model.temperature_start,
            temperature_end: model.temperature_end,
            is_calibration: model.is_calibration,
            remarks: model.remarks,
            assets: vec![],
            regions: vec![],
            results_summary: None,
        }
    }
}

#[async_trait]
impl CRUDResource for Experiment {
    type EntityType = crate::routes::experiments::models::Entity;
    type ColumnType = crate::routes::experiments::models::Column;
    type ActiveModelType = crate::routes::experiments::models::ActiveModel;
    type CreateModel = ExperimentCreate;
    type UpdateModel = ExperimentUpdate;
    type ListModel = Self; // Use the same model for list view for now

    const ID_COLUMN: Self::ColumnType = crate::routes::experiments::models::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "experiments";
    const RESOURCE_NAME_SINGULAR: &'static str = "experiment";
    const RESOURCE_DESCRIPTION: &'static str =
        "This resource manages experiments associated with sample data collected during campaigns.";

    async fn get_one(db: &DatabaseConnection, id: Uuid) -> Result<Self, DbErr> {
        let model =
            Self::EntityType::find_by_id(id)
                .one(db)
                .await?
                .ok_or(DbErr::RecordNotFound(format!(
                    "{} not found",
                    Self::RESOURCE_NAME_SINGULAR
                )))?;

        let s3_assets = model
            .find_related(crate::routes::assets::models::Entity)
            .all(db)
            .await?;

        let regions = model
            .find_related(crate::routes::trays::regions::models::Entity)
            .all(db)
            .await?;

        let mut regions_with_treatment = Vec::new();
        for region in regions {
            let region_input = region_model_to_input_with_treatment(region, db).await?;
            regions_with_treatment.push(region_input);
        }

        // Build results summary
        let results_summary = build_results_summary(id, db).await?;

        let mut model: Self = model.into();
        model.assets = s3_assets.into_iter().map(Into::into).collect();
        model.regions = regions_with_treatment;
        model.results_summary = results_summary;

        Ok(model)
    }

    async fn create(db: &DatabaseConnection, data: Self::CreateModel) -> Result<Self, DbErr> {
        let txn = db.begin().await?;

        // Store regions before conversion since they're not part of the DB model
        let regions_to_create = data.regions.clone();

        // Create the experiment first
        let experiment_model: Self::ActiveModelType = data.into();
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
        Self::get_one(db, experiment.id).await
    }

    async fn update(
        db: &DatabaseConnection,
        id: Uuid,
        update_data: Self::UpdateModel,
    ) -> Result<Self, DbErr> {
        let txn = db.begin().await?;

        let existing: Self::ActiveModelType = Self::EntityType::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_PLURAL
            )))?
            .into();
        let regions = update_data.regions.clone();
        let updated_model = update_data.merge_into_activemodel(existing)?;
        let _updated = updated_model.update(&txn).await?;

        // Handle regions update - delete existing regions and create new ones
        if !regions.is_empty() {
            // Delete existing regions for this experiment
            crate::routes::trays::regions::models::Entity::delete_many()
                .filter(crate::routes::trays::regions::models::Column::ExperimentId.eq(id))
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
        Self::get_one(db, id).await
    }

    async fn get_all(
        db: &DatabaseConnection,
        condition: &Condition,
        order_column: Self::ColumnType,
        order_direction: Order,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Self>, DbErr> {
        let models = Self::EntityType::find()
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
                .find_related(crate::routes::trays::regions::models::Entity)
                .all(db)
                .await?;

            let mut regions_with_treatment = Vec::new();
            for region in regions {
                let region_input = region_model_to_input_with_treatment(region, db).await?;
                regions_with_treatment.push(region_input);
            }

            let mut experiment: Self = model.into();
            experiment.assets = s3_assets.into_iter().map(Into::into).collect();
            experiment.regions = regions_with_treatment;

            experiments.push(experiment);
        }

        Ok(experiments)
    }

    fn sortable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("id", Self::ColumnType::Id),
            ("name", Self::ColumnType::Name),
            ("performed_at", Self::ColumnType::PerformedAt),
            ("username", Self::ColumnType::Username),
            ("created_at", Self::ColumnType::CreatedAt),
            ("temperature_ramp", Self::ColumnType::TemperatureRamp),
            ("temperature_start", Self::ColumnType::TemperatureStart),
            ("temperature_end", Self::ColumnType::TemperatureEnd),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("name", Self::ColumnType::Name),
            ("performed_at", Self::ColumnType::PerformedAt),
            ("username", Self::ColumnType::Username),
            ("created_at", Self::ColumnType::CreatedAt),
            ("temperature_ramp", Self::ColumnType::TemperatureRamp),
            ("temperature_start", Self::ColumnType::TemperatureStart),
            ("temperature_end", Self::ColumnType::TemperatureEnd),
        ]
    }
}
