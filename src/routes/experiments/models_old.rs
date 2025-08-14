use super::services::{
    build_results_summary, create_region_active_models, region_model_to_input_with_treatment,
};
use crate::routes::experiments::models::Model;
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
    pub image_filename_at_freeze: Option<String>, // Image filename at time of first phase change (without .jpg extension)
    pub image_asset_id: Option<Uuid>, // Asset ID for the image at freeze time (for secure viewing via /assets/{id}/view)
    pub tray_id: Option<String>, // UUID of the tray
    pub tray_name: Option<String>,
    pub dilution_factor: Option<i32>,
    // Full objects with UUIDs for UI linking
    // pub treatment: Option<crate::routes::treatments::models::TreatmentList>,
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
