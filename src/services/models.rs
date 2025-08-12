use crate::routes::treatments::models::TreatmentName;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Common temperature reading structure used across different processing contexts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TemperatureReading {
    pub probe_sequence: i32,
    pub temperature: f64,
}

/// Common well state structure for phase transition tracking  
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WellState {
    pub row: i32,
    pub col: i32,
    pub value: i32,
}

/// Common treatment information used across entities
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TreatmentInfo {
    pub id: Uuid,
    pub name: TreatmentName,
    pub notes: Option<String>,
    pub enzyme_volume_litres: Option<Decimal>,
}

impl From<crate::routes::treatments::models::Model> for TreatmentInfo {
    fn from(model: crate::routes::treatments::models::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            notes: model.notes,
            enzyme_volume_litres: model.enzyme_volume_litres,
        }
    }
}

/// Common pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationParams {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(50),
        }
    }
}

/// Common response wrapper for paginated results  
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
    pub total_pages: u64,
}

/// Common error response structure
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

/// Processing status for async operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Common processing result wrapper
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProcessingResult<T> {
    pub status: ProcessingStatus,
    pub data: Option<T>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
