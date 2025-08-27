//! Excel processor for SPICE experiment files
//!
//! This module provides Excel file processing functionality for ice nucleation experiments.
//! It handles parsing Excel files with complex header structures and extracting temperature
//! and phase transition data for storage in the database.

use crate::common::models::ProcessingStatus;
use anyhow::Result;
use chrono::Utc;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use uuid::Uuid;

use super::{
    database::{DatabaseOperations, ProcessingBatches},
    row_processing::{ProcessingResult, process_row},
    structure::parse_excel_structure,
    utils::load_excel,
};

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

/// Service for Excel data processing operations
#[derive(Clone)]
pub struct ExcelProcessor {
    db: DatabaseConnection,
}

impl ExcelProcessor {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Clear existing experimental data for an experiment before reprocessing
    async fn clear_experiment_data(&self, experiment_id: Uuid) -> Result<()> {
        use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

        // Delete phase transitions for this experiment first
        crate::experiments::phase_transitions::models::Entity::delete_many()
            .filter(crate::experiments::phase_transitions::models::Column::ExperimentId.eq(experiment_id))
            .exec(&self.db)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to clear phase transitions: {}", e))?;

        // Delete temperature readings for this experiment (will cascade delete probe readings due to FK constraints)
        crate::experiments::temperatures::models::Entity::delete_many()
            .filter(crate::experiments::temperatures::models::Column::ExperimentId.eq(experiment_id))
            .exec(&self.db)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to clear temperature readings: {}", e))?;

        Ok(())
    }

    /// Process Excel file for an experiment
    pub async fn process_excel_file(
        &self,
        experiment_id: Uuid,
        file_data: Vec<u8>,
    ) -> Result<ExcelProcessingResult> {
        let started_at = Utc::now();

        match self
            .process_excel_file_direct(file_data, experiment_id)
            .await
        {
            Ok(result) => Ok(ExcelProcessingResult {
                status: ProcessingStatus::Completed,
                success: result.success,
                temperature_readings_created: result.temperature_readings,
                probe_temperature_readings_created: result.probe_readings,
                phase_transitions_created: result.phase_transitions,
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

    /// Process Excel file for an experiment (internal implementation)
    async fn process_excel_file_direct(
        &self,
        file_data: Vec<u8>,
        experiment_id: Uuid,
    ) -> Result<ProcessingResult> {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();

        // Clear existing experimental data before processing to avoid duplicates
        self.clear_experiment_data(experiment_id).await?;

        // Load Excel data and parse structure
        let rows = load_excel(file_data)?;
        let structure = parse_excel_structure(&rows)?;

        // Initialize database operations
        let db_ops = DatabaseOperations::new(self.db.clone());

        // Get tray mappings and ensure wells exist
        let tray_mappings = db_ops.load_tray_mappings(experiment_id).await?;
        db_ops
            .ensure_wells_exist(&structure, &tray_mappings)
            .await?;

        // Load mappings in parallel
        let (well_mappings, probe_mappings) = tokio::join!(
            db_ops.load_well_mappings(&structure, experiment_id),
            db_ops.load_probe_mappings(experiment_id)
        );
        let well_mappings = well_mappings?;
        let probe_mappings = probe_mappings?;

        if well_mappings.is_empty() {
            return Err(anyhow::anyhow!("No wells found for experiment"));
        }

        // Process data in batches
        let mut batches = ProcessingBatches::default();
        let mut phase_states: HashMap<String, i32> = HashMap::new();

        for (row_idx, row) in rows.iter().skip(structure.data_start_row).enumerate() {
            match process_row(
                row,
                &structure,
                experiment_id,
                &well_mappings,
                &probe_mappings,
                &mut phase_states,
            ) {
                Ok((temp_reading, probe_readings, transitions)) => {
                    if let Some(tr) = temp_reading {
                        batches.temp_readings.push(tr);
                    }
                    batches.probe_readings.extend(probe_readings);
                    batches.phase_transitions.extend(transitions);

                    // Batch insert every 500 records
                    if batches.total_count() >= 500 {
                        batches.flush(&self.db).await?;
                    }
                }
                Err(e) => {
                    errors.push(format!(
                        "Row {}: {e}",
                        row_idx + structure.data_start_row + 1,
                    ));
                    if errors.len() > 20 {
                        break;
                    }
                }
            }
        }

        // Final flush
        batches.flush(&self.db).await?;

        let processing_time = start_time.elapsed().as_millis();

        Ok(ProcessingResult {
            success: errors.len() < 10,
            temperature_readings: batches.temp_readings_total,
            probe_readings: batches.probe_readings_total,
            phase_transitions: batches.phase_transitions_total,
            wells_tracked: structure.well_columns.len(),
            errors,
            processing_time_ms: processing_time,
        })
    }
}

// Re-exports for API compatibility
pub use ExcelProcessor as DataProcessingService;
