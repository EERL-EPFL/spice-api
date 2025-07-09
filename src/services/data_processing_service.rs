use anyhow::Result;
use chrono::Utc;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use super::processing::DirectExcelProcessor;
use crate::services::models::ProcessingStatus;

/// Service for data processing operations (Excel upload, phase changes, results generation)
#[derive(Clone)]
pub struct DataProcessingService {
    db: DatabaseConnection,
}

impl DataProcessingService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Process Excel file for an experiment
    pub async fn process_excel_file(
        &self,
        experiment_id: Uuid,
        file_data: Vec<u8>,
    ) -> Result<ExcelProcessingResult> {
        let started_at = Utc::now();

        let mut processor = DirectExcelProcessor::new(self.db.clone());

        match processor
            .process_excel_file_direct(file_data, experiment_id)
            .await
        {
            Ok(result) => Ok(ExcelProcessingResult {
                status: ProcessingStatus::Completed,
                time_points_created: result.time_points_created,
                temperature_readings_created: result.temperature_readings_created,
                well_states_created: result.well_states_created,
                processing_time_ms: result.processing_time_ms,
                started_at,
                completed_at: Some(Utc::now()),
                error: None,
                errors: result.errors,
            }),
            Err(e) => Ok(ExcelProcessingResult {
                status: ProcessingStatus::Failed,
                time_points_created: 0,
                temperature_readings_created: 0,
                well_states_created: 0,
                processing_time_ms: 0,
                started_at,
                completed_at: Some(Utc::now()),
                error: Some(e.to_string()),
                errors: vec![e.to_string()],
            }),
        }
    }
}

/// Result of Excel file processing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ExcelProcessingResult {
    pub status: ProcessingStatus,
    pub time_points_created: usize,
    pub temperature_readings_created: usize,
    pub well_states_created: usize,
    pub processing_time_ms: u128,
    pub started_at: chrono::DateTime<Utc>,
    pub completed_at: Option<chrono::DateTime<Utc>>,
    pub error: Option<String>,
    pub errors: Vec<String>,
}
