use super::processing::DirectExcelProcessor;
use crate::common::models::ProcessingStatus;
use anyhow::Result;
use chrono::Utc;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

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
                success: result.success,
                temperature_readings_created: result.temperature_readings_created,
                probe_temperature_readings_created: result.probe_temperature_readings_created,
                phase_transitions_created: result.phase_transitions_created,
                wells_tracked: result.wells_tracked,
                processing_time_ms: result.processing_time_ms,
                started_at,
                completed_at: Some(Utc::now()),
                error: None,
                errors: result.errors,
            }),
            Err(e) => Ok(ExcelProcessingResult {
                status: ProcessingStatus::Failed,
                success: false,
                temperature_readings_created: 0,
                probe_temperature_readings_created: 0,
                phase_transitions_created: 0,
                wells_tracked: 0,
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
    pub success: bool,
    pub temperature_readings_created: usize,
    pub probe_temperature_readings_created: usize,
    pub phase_transitions_created: usize,
    pub wells_tracked: usize,
    pub processing_time_ms: u128,
    pub started_at: chrono::DateTime<Utc>,
    pub completed_at: Option<chrono::DateTime<Utc>>,
    pub error: Option<String>,
    pub errors: Vec<String>,
}
