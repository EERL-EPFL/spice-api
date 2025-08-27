//! Excel processor for SPICE experiment files
//!
//! This module provides Excel file processing functionality for ice nucleation experiments.
//! It handles parsing Excel files with complex header structures and extracting temperature
//! and phase transition data for storage in the database.

use crate::{
    common::models::ProcessingStatus,
    experiments::{
        models as experiments, phase_transitions::models as phase_transitions,
        probe_temperature_readings::models as probe_temperature_readings,
        temperatures::models as temperature_readings,
    },
    tray_configurations::{
        probes::models as probes, trays::models as tray_configuration_assignments,
        wells::models as wells,
    },
};
use anyhow::{Context, Result, anyhow};
use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};
use chrono::{NaiveDateTime, Timelike, Utc};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use std::io::Cursor;
use uuid::Uuid;

/// Excel structure representation
#[derive(Debug)]
pub struct ExcelStructure {
    pub date_col: usize,
    pub time_col: usize,
    pub image_col: Option<usize>,
    pub well_columns: HashMap<String, usize>, // "P1:A1" -> column_index
    pub probe_columns: Vec<usize>,
    pub data_start_row: usize,
}

/// Processing result structure
#[derive(Debug)]
pub struct ProcessingResult {
    pub success: bool,
    pub temperature_readings: usize,
    pub probe_readings: usize,
    pub phase_transitions: usize,
    pub wells_tracked: usize,
    pub errors: Vec<String>,
    pub processing_time_ms: u128,
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

/// Service for Excel data processing operations
#[derive(Clone)]
pub struct ExcelProcessor {
    db: DatabaseConnection,
}

impl ExcelProcessor {
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

        // Load Excel data
        let rows = Self::load_excel(file_data)?;
        let structure = parse_excel_structure(&rows)?;

        // Get tray mappings and ensure wells exist
        let tray_mappings = self.load_tray_mappings(experiment_id).await?;
        self.ensure_wells_exist(&structure, &tray_mappings).await?;

        // Load mappings in parallel
        let (well_mappings, probe_mappings) = tokio::join!(
            self.load_well_mappings(&structure, experiment_id),
            self.load_probe_mappings(experiment_id)
        );
        let well_mappings = well_mappings?;
        let probe_mappings = probe_mappings?;

        if well_mappings.is_empty() {
            return Err(anyhow!("No wells found for experiment"));
        }

        // Process data in batches
        let mut batches = ProcessingBatches::default();
        let mut phase_states: HashMap<String, i32> = HashMap::new();

        for (row_idx, row) in rows.iter().skip(structure.data_start_row).enumerate() {
            match self.process_row(
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
                        self.flush_batches(&mut batches).await?;
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
        self.flush_batches(&mut batches).await?;

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

    fn load_excel(file_data: Vec<u8>) -> Result<Vec<Vec<Data>>> {
        let cursor = Cursor::new(file_data);
        let mut workbook: Xlsx<_> = open_workbook_from_rs(cursor)?;
        let sheet_name = workbook
            .sheet_names()
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No worksheets"))?;
        let worksheet = workbook.worksheet_range(&sheet_name)?;
        Ok(worksheet.rows().map(<[Data]>::to_vec).collect())
    }

    async fn load_well_mappings(
        &self,
        structure: &ExcelStructure,
        experiment_id: Uuid,
    ) -> Result<HashMap<String, Uuid>> {
        // Get experiment's tray configuration
        let experiment = experiments::Entity::find_by_id(experiment_id)
            .one(&self.db)
            .await
            .context("Failed to query experiment")?
            .ok_or_else(|| anyhow!("Experiment not found"))?;

        let tray_configuration_id = experiment
            .tray_configuration_id
            .ok_or_else(|| anyhow!("Experiment has no tray configuration"))?;

        // Load tray name to ID mapping
        let tray_assignments = tray_configuration_assignments::Entity::find()
            .filter(
                tray_configuration_assignments::Column::TrayConfigurationId
                    .eq(tray_configuration_id),
            )
            .all(&self.db)
            .await
            .context("Failed to query tray assignments")?;

        let mut tray_name_to_id: HashMap<String, Uuid> = HashMap::new();
        for assignment in &tray_assignments {
            if let Some(ref name) = assignment.name {
                tray_name_to_id.insert(name.clone(), assignment.id);
            }
        }

        // Load well mappings
        let mut well_mappings = HashMap::new();

        for well_key in structure.well_columns.keys() {
            // Parse well_key like "P1:A1"
            if let Some((tray_name, well_coord)) = well_key.split_once(':') {
                if let Some(&tray_id) = tray_name_to_id.get(tray_name) {
                    // Parse coordinate like "A1" -> row_letter="A", column_number=1
                    if let Ok((row_letter, column_number)) = Self::parse_well_coordinate(well_coord)
                    {
                        // Find the well in the database
                        let well = wells::Entity::find()
                            .filter(wells::Column::TrayId.eq(tray_id))
                            .filter(wells::Column::RowLetter.eq(&row_letter))
                            .filter(wells::Column::ColumnNumber.eq(column_number))
                            .one(&self.db)
                            .await
                            .context("Failed to query well")?;

                        if let Some(well) = well {
                            well_mappings.insert(well_key.clone(), well.id);
                        } else {
                            tracing::warn!(
                                "Well not found: tray={tray_name}, row={row_letter}, col={column_number}"
                            );
                        }
                    } else {
                        tracing::warn!("Invalid coordinate: {well_coord}");
                    }
                }
            }
        }

        tracing::debug!("Loaded {} well mappings from database", well_mappings.len());
        Ok(well_mappings)
    }

    fn parse_well_coordinate(coord: &str) -> Result<(String, i32)> {
        if coord.is_empty() {
            return Err(anyhow!("Empty coordinate"));
        }

        let mut chars = coord.chars();
        let row_letter = chars.next().unwrap().to_string();
        let column_str: String = chars.collect();
        let column_number = column_str
            .parse::<i32>()
            .map_err(|_| anyhow!("Invalid column number: {coord}"))?;

        Ok((row_letter, column_number))
    }

    async fn load_probe_mappings(&self, experiment_id: Uuid) -> Result<HashMap<usize, Uuid>> {
        // Get experiment's tray configuration
        let experiment = experiments::Entity::find_by_id(experiment_id)
            .one(&self.db)
            .await
            .context("Failed to query experiment")?
            .ok_or_else(|| anyhow!("Experiment not found"))?;

        let tray_configuration_id = experiment
            .tray_configuration_id
            .ok_or_else(|| anyhow!("Experiment has no tray configuration"))?;

        // Get all trays for this configuration first
        let tray_records = tray_configuration_assignments::Entity::find()
            .filter(
                tray_configuration_assignments::Column::TrayConfigurationId
                    .eq(tray_configuration_id),
            )
            .all(&self.db)
            .await
            .context("Failed to query trays")?;

        // Load probes for all trays in the configuration
        let mut probe_mappings = HashMap::new();
        for tray in &tray_records {
            let probe_records = probes::Entity::find()
                .filter(probes::Column::TrayId.eq(tray.id))
                .all(&self.db)
                .await
                .context("Failed to query probes")?;

            for probe in &probe_records {
                // Map data_column_index to probe ID with proper cast handling
                #[allow(clippy::cast_sign_loss)]
                // data_column_index is always positive in this context
                let col_index = probe.data_column_index as usize;
                probe_mappings.insert(col_index, probe.id);
            }
        }

        tracing::debug!(
            "Loaded {} probe mappings from database",
            probe_mappings.len()
        );
        Ok(probe_mappings)
    }

    async fn load_tray_mappings(&self, experiment_id: Uuid) -> Result<HashMap<String, Uuid>> {
        // Get experiment and its tray configuration
        let experiment = experiments::Entity::find_by_id(experiment_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow!("Experiment not found"))?;

        let tray_configuration_id = experiment
            .tray_configuration_id
            .ok_or_else(|| anyhow!("Experiment has no tray configuration"))?;

        // Get tray assignments
        let assignments = tray_configuration_assignments::Entity::find()
            .filter(
                tray_configuration_assignments::Column::TrayConfigurationId
                    .eq(tray_configuration_id),
            )
            .all(&self.db)
            .await?;

        let mut tray_mappings = HashMap::new();
        for assignment in assignments {
            if let Some(name) = assignment.name {
                tray_mappings.insert(name, assignment.id);
            }
        }

        tracing::debug!("Loaded {} tray mappings from database", tray_mappings.len());
        Ok(tray_mappings)
    }

    async fn ensure_wells_exist(
        &self,
        structure: &ExcelStructure,
        tray_mappings: &HashMap<String, Uuid>,
    ) -> Result<()> {
        for (tray_name, &tray_id) in tray_mappings {
            self.ensure_tray_wells_exist(structure, tray_name, tray_id)
                .await?;
        }
        Ok(())
    }

    async fn ensure_tray_wells_exist(
        &self,
        structure: &ExcelStructure,
        tray_name: &str,
        tray_id: Uuid,
    ) -> Result<()> {
        let existing_wells: Vec<wells::Model> = wells::Entity::find()
            .filter(wells::Column::TrayId.eq(tray_id))
            .all(&self.db)
            .await
            .context("Failed to query existing wells")?;

        // Extract wells for this tray from the Excel structure
        let wells_for_tray: Vec<(&str, &str)> = structure
            .well_columns
            .iter()
            .filter_map(|(well_key, _)| {
                // well_key format: "P1:A1"
                let parts: Vec<&str> = well_key.split(':').collect();
                if parts.len() == 2 && parts[0] == tray_name {
                    Some((parts[0], parts[1]))
                } else {
                    None
                }
            })
            .collect();

        if existing_wells.is_empty() && !wells_for_tray.is_empty() {
            tracing::info!("Creating wells for tray {tray_name}");
            self.create_wells_from_excel_headers(tray_id, &wells_for_tray)
                .await?;
        } else {
            tracing::debug!(
                "Tray {tray_name} has {} existing wells",
                existing_wells.len()
            );
        }
        Ok(())
    }

    async fn create_wells_from_excel_headers(
        &self,
        tray_id: Uuid,
        wells_for_tray: &[(&str, &str)], // (tray_name, coordinate)
    ) -> Result<()> {
        let mut wells_data = Vec::new();

        for (_, coord) in wells_for_tray {
            // Parse coordinate like "A1" into row letter and column number
            let row_letter = coord
                .chars()
                .take_while(char::is_ascii_alphabetic)
                .collect::<String>();
            let col_str: String = coord
                .chars()
                .skip_while(char::is_ascii_alphabetic)
                .collect();
            let column_number: i32 = col_str.parse().context("Invalid column number")?;

            let well = wells::ActiveModel {
                id: Set(Uuid::new_v4()),
                tray_id: Set(tray_id),
                row_letter: Set(row_letter),
                column_number: Set(column_number),
                created_at: Set(chrono::Utc::now()),
                last_updated: Set(chrono::Utc::now()),
            };
            wells_data.push(well);
        }

        if !wells_data.is_empty() {
            wells::Entity::insert_many(wells_data)
                .exec(&self.db)
                .await
                .context("Failed to batch insert wells from Excel headers")?;

            tracing::info!("Created {} wells for tray", wells_for_tray.len());
        }

        Ok(())
    }

    #[allow(clippy::unused_self)] // Part of struct methods for consistency
    fn process_row(
        &self,
        row: &[Data],
        structure: &ExcelStructure,
        experiment_id: Uuid,
        well_mappings: &HashMap<String, Uuid>,
        probe_mappings: &HashMap<usize, Uuid>,
        phase_states: &mut HashMap<String, i32>,
    ) -> Result<(
        Option<temperature_readings::ActiveModel>,
        Vec<probe_temperature_readings::ActiveModel>,
        Vec<phase_transitions::ActiveModel>,
    )> {
        // Extract timestamp
        let timestamp = Self::parse_timestamp(row, structure)?;
        let timestamp_clean = timestamp.with_nanosecond(0).unwrap_or(timestamp);

        // Create temperature reading
        let temp_reading = temperature_readings::ActiveModel {
            id: Set(Uuid::new_v4()),
            experiment_id: Set(experiment_id),
            timestamp: Set(timestamp_clean),
            image_filename: Set(Self::extract_image_filename(row, structure)),
            created_at: Set(Utc::now()),
        };

        // Create probe readings
        let mut probe_readings = Vec::new();
        for &probe_col in &structure.probe_columns {
            if let (Some(cell), Some(&probe_id)) =
                (row.get(probe_col), probe_mappings.get(&probe_col))
            {
                if let Some(temp) = extract_decimal(cell) {
                    probe_readings.push(probe_temperature_readings::ActiveModel {
                        id: Set(Uuid::new_v4()),
                        temperature_reading_id: Set(*temp_reading.id.as_ref()),
                        probe_id: Set(probe_id),
                        temperature: Set(temp),
                        created_at: Set(Utc::now()),
                    });
                }
            }
        }

        // Process phase transitions
        let mut transitions = Vec::new();
        for (well_key, &col_idx) in &structure.well_columns {
            if let Some(cell) = row.get(col_idx) {
                if let Some(new_phase) = extract_integer(cell) {
                    let previous = phase_states.get(well_key).copied().unwrap_or(0);
                    phase_states.insert(well_key.clone(), new_phase);

                    if previous != new_phase {
                        if let Some(&well_id) = well_mappings.get(well_key) {
                            transitions.push(phase_transitions::ActiveModel {
                                id: Set(Uuid::new_v4()),
                                well_id: Set(well_id),
                                experiment_id: Set(experiment_id),
                                temperature_reading_id: Set(*temp_reading.id.as_ref()),
                                timestamp: Set(timestamp_clean),
                                previous_state: Set(previous),
                                new_state: Set(new_phase),
                                created_at: Set(Utc::now()),
                            });
                        }
                    }
                }
            }
        }

        Ok((Some(temp_reading), probe_readings, transitions))
    }

    fn parse_timestamp(row: &[Data], structure: &ExcelStructure) -> Result<chrono::DateTime<Utc>> {
        let date_cell = row
            .get(structure.date_col)
            .ok_or_else(|| anyhow!("Missing date"))?;
        let time_cell = row
            .get(structure.time_col)
            .ok_or_else(|| anyhow!("Missing time"))?;

        match (date_cell, time_cell) {
            (Data::String(date_str), Data::String(time_str)) => {
                let combined = format!("{date_str} {time_str}");

                // Try multiple datetime formats
                if let Ok(dt) = NaiveDateTime::parse_from_str(&combined, "%Y-%m-%d %H:%M:%S") {
                    Ok(dt.and_utc())
                } else if let Ok(dt) = NaiveDateTime::parse_from_str(&combined, "%m/%d/%Y %H:%M:%S")
                {
                    Ok(dt.and_utc())
                } else if let Ok(dt) =
                    NaiveDateTime::parse_from_str(&combined, "%Y-%m-%d %H:%M:%S%.f")
                {
                    Ok(dt.and_utc())
                } else if let Ok(dt) =
                    NaiveDateTime::parse_from_str(&combined, "%m/%d/%Y %H:%M:%S%.f")
                {
                    Ok(dt.and_utc())
                } else {
                    Err(anyhow!("Could not parse datetime: {combined}"))
                }
            }
            (Data::DateTime(excel_dt), _) => {
                // Use calamine's Excel date as float and convert
                let timestamp_secs = (excel_dt.as_f64() - 25569.0) * 86400.0; // Excel epoch to Unix epoch

                // Check bounds more precisely for f64 -> i64 conversion
                if timestamp_secs.is_finite() {
                    // Safe cast: checked that value is finite above
                    #[allow(clippy::cast_possible_truncation)]
                    let timestamp_int = timestamp_secs as i64;
                    Ok(chrono::DateTime::from_timestamp(timestamp_int, 0)
                        .ok_or_else(|| anyhow!("Invalid timestamp: {}", timestamp_secs))?)
                } else {
                    Err(anyhow!("Excel timestamp is not finite: {}", timestamp_secs))
                }
            }
            (Data::Float(timestamp), _) => {
                // Handle float timestamp as Unix timestamp
                let rounded_timestamp = timestamp.round();

                // Check if finite before converting
                if rounded_timestamp.is_finite() {
                    // Safe cast: checked that value is finite above
                    #[allow(clippy::cast_possible_truncation)]
                    let timestamp_int = rounded_timestamp as i64;
                    Ok(chrono::DateTime::from_timestamp(timestamp_int, 0)
                        .ok_or_else(|| anyhow!("Invalid timestamp: {}", rounded_timestamp))?
                        .with_timezone(&chrono::Utc))
                } else {
                    Err(anyhow!(
                        "Float timestamp is not finite: {}",
                        rounded_timestamp
                    ))
                }
            }
            _ => Err(anyhow!(
                "Unsupported timestamp format: {date_cell:?}, {time_cell:?}"
            )),
        }
    }

    fn extract_image_filename(row: &[Data], structure: &ExcelStructure) -> Option<String> {
        structure
            .image_col
            .and_then(|col| row.get(col))
            .and_then(|cell| match cell {
                Data::String(s) => Some(s.clone()),
                _ => None,
            })
    }

    async fn flush_batches(&self, batches: &mut ProcessingBatches) -> Result<()> {
        // Update totals before draining
        batches.temp_readings_total += batches.temp_readings.len();
        batches.probe_readings_total += batches.probe_readings.len();
        batches.phase_transitions_total += batches.phase_transitions.len();

        // Insert batches
        if !batches.temp_readings.is_empty() {
            temperature_readings::Entity::insert_many(batches.temp_readings.drain(..))
                .exec(&self.db)
                .await?;
        }
        if !batches.probe_readings.is_empty() {
            probe_temperature_readings::Entity::insert_many(batches.probe_readings.drain(..))
                .exec(&self.db)
                .await?;
        }
        if !batches.phase_transitions.is_empty() {
            phase_transitions::Entity::insert_many(batches.phase_transitions.drain(..))
                .exec(&self.db)
                .await?;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct ProcessingBatches {
    temp_readings: Vec<temperature_readings::ActiveModel>,
    probe_readings: Vec<probe_temperature_readings::ActiveModel>,
    phase_transitions: Vec<phase_transitions::ActiveModel>,
    temp_readings_total: usize,
    probe_readings_total: usize,
    phase_transitions_total: usize,
}

impl ProcessingBatches {
    fn total_count(&self) -> usize {
        self.temp_readings.len() + self.probe_readings.len() + self.phase_transitions.len()
    }
}

/// Parse Excel structure from raw rows
fn parse_excel_structure(rows: &[Vec<Data>]) -> Result<ExcelStructure> {
    if rows.len() < 7 {
        return Err(anyhow!("Invalid Excel format"));
    }

    let tray_row = &rows[0];
    let coord_row = &rows[1];
    let header_row = &rows[6];

    let mut well_columns = HashMap::new();
    let mut probe_columns = Vec::new();
    let mut date_col = None;
    let mut time_col = None;
    let mut image_col = None;

    // Single pass through headers
    for (col_idx, cell) in header_row.iter().enumerate() {
        if let Data::String(header) = cell {
            match header.as_str() {
                "Date" => date_col = Some(col_idx),
                "Time" => time_col = Some(col_idx),
                h if h.contains(".jpg") => image_col = Some(col_idx),
                h if h.starts_with("Temperature") => probe_columns.push(col_idx),
                "()" => {
                    // Well column - get tray and coordinate
                    if let (Some(Data::String(tray)), Some(Data::String(coord))) =
                        (tray_row.get(col_idx), coord_row.get(col_idx))
                    {
                        if (tray == "P1" || tray == "P2") && is_valid_coord(coord) {
                            well_columns.insert(format!("{tray}:{coord}"), col_idx);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(ExcelStructure {
        date_col: date_col.ok_or_else(|| anyhow!("No date column"))?,
        time_col: time_col.ok_or_else(|| anyhow!("No time column"))?,
        image_col,
        well_columns,
        probe_columns,
        data_start_row: 7,
    })
}

fn is_valid_coord(coord: &str) -> bool {
    coord.len() >= 2
        && coord.chars().next().unwrap().is_ascii_uppercase()
        && coord.chars().skip(1).all(|c| c.is_ascii_digit())
}

fn extract_decimal(cell: &Data) -> Option<Decimal> {
    match cell {
        Data::Float(f) => Decimal::from_f64_retain(*f),
        Data::Int(i) => Some(Decimal::from(*i)),
        _ => None,
    }
}

fn extract_integer(cell: &Data) -> Option<i32> {
    match cell {
        Data::Int(i) => i32::try_from(*i).ok(),
        Data::Float(f) => {
            let rounded = f.round();
            // Check if finite and within i32 range
            if rounded.is_finite()
                && rounded >= f64::from(i32::MIN)
                && rounded <= f64::from(i32::MAX)
            {
                // Safe cast: checked bounds and finiteness above
                #[allow(clippy::cast_possible_truncation)]
                Some(rounded as i32)
            } else {
                None // Return None for out-of-range or non-finite values
            }
        }
        _ => None,
    }
}

// Legacy aliases for backwards compatibility
pub use ExcelProcessor as DataProcessingService;

// For backwards compatibility with existing imports
#[allow(unused)]
pub use ExcelProcessor as DirectExcelProcessor;

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::Data;

    #[test]
    fn test_excel_structure_parsing() {
        let test_data = vec![
            // Row 1: Tray names
            vec![
                Data::String("Date".to_string()),
                Data::String("Time".to_string()),
                Data::String("P1".to_string()),
                Data::String("P2".to_string()),
            ],
            // Row 2: Well coordinates
            vec![
                Data::String(String::new()),
                Data::String(String::new()),
                Data::String("A1".to_string()),
                Data::String("A1".to_string()),
            ],
            // Rows 3-6: Empty
            vec![Data::String(String::new()); 4],
            vec![Data::String(String::new()); 4],
            vec![Data::String(String::new()); 4],
            vec![Data::String(String::new()); 4],
            // Row 7: Column headers
            vec![
                Data::String("Date".to_string()),
                Data::String("Time".to_string()),
                Data::String("()".to_string()),
                Data::String("()".to_string()),
            ],
        ];

        let result = parse_excel_structure(&test_data);
        assert!(result.is_ok(), "Should parse valid Excel structure");

        let structure = result.unwrap();
        assert_eq!(structure.date_col, 0);
        assert_eq!(structure.time_col, 1);
        assert_eq!(structure.well_columns.len(), 2);
        assert_eq!(structure.data_start_row, 7);
    }

    #[test]
    fn test_coordinate_validation() {
        // Test coordinate parsing logic
        assert!(is_valid_coord("A1"));
        assert!(is_valid_coord("H12"));
        assert!(is_valid_coord("Z99"));

        assert!(!is_valid_coord(""));
        assert!(!is_valid_coord("1A"));
        assert!(!is_valid_coord("AA")); // No number
    }

    #[test]
    fn test_invalid_excel_format() {
        let insufficient_data = vec![
            vec![Data::String("P1".to_string())],
            vec![Data::String("A1".to_string())],
        ];

        let result = parse_excel_structure(&insufficient_data);
        assert!(result.is_err(), "Should reject insufficient data");
    }
}
