use crate::routes::trays::services::str_to_coordinates;
use anyhow::{Context, Result, anyhow};
use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};
use chrono::{NaiveDateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use spice_entity::{
    experiments, temperature_readings, tray_configuration_assignments, trays,
    well_phase_transitions, wells,
};
use std::collections::HashMap;
use std::io::Cursor;
use uuid::Uuid;

const CHUNK_SIZE: usize = 500;
#[derive(Debug)]
pub struct DirectExcelProcessor {
    db: DatabaseConnection,
    well_states: HashMap<String, i32>, // Track previous state for each well (tray_name:well_coordinate -> phase)
    well_ids: HashMap<String, Uuid>,   // Map tray_name:well_coordinate -> well_id
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DirectProcessingResult {
    pub success: bool,
    pub temperature_readings_created: usize,
    pub phase_transitions_created: usize,
    pub well_states_created: usize,
    pub time_points_created: usize,
    pub wells_tracked: usize,
    pub errors: Vec<String>,
    pub processing_time_ms: u128,
}

#[derive(Debug, Clone)]
struct WellMapping {
    col_idx: usize,
    row: i32,
    col: i32,
    tray_name: String,
    well_coordinate: String,
}

#[derive(Debug)]
struct ColumnHeaders {
    date_col: Option<usize>,
    time_col: Option<usize>,
    image_col: Option<usize>,
    temp_cols: [Option<usize>; 8],
    wells: Vec<WellMapping>,
}

impl DirectExcelProcessor {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            well_states: HashMap::new(),
            well_ids: HashMap::new(),
        }
    }

    pub async fn process_excel_file_direct(
        &mut self,
        file_data: Vec<u8>,
        experiment_id: Uuid,
    ) -> Result<DirectProcessingResult> {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();

        println!("🚀 Starting direct Excel processing (phase transition tracking)");

        // 1. Load the xlsx
        let cursor = Cursor::new(file_data);
        let mut workbook: Xlsx<_> = open_workbook_from_rs(cursor)
            .map_err(|e| anyhow!("Failed to open Excel workbook: {}", e))?;

        let sheet_names = workbook.sheet_names();
        let sheet_name = sheet_names
            .first()
            .ok_or_else(|| anyhow!("No worksheets found in Excel file"))?;
        let worksheet = workbook
            .worksheet_range(sheet_name)
            .map_err(|e| anyhow!("Failed to read worksheet: {}", e))?;

        let rows: Vec<_> = worksheet.rows().collect();
        if rows.len() < 8 {
            return Err(anyhow!("Excel file format invalid: need at least 8 rows"));
        }
        println!("   📊 Total rows in Excel: {}", rows.len());

        let headers = self.parse_headers(&rows)?; // Identify tray and well headers from first two rows, then probe/time headers
        println!(
            "   🗺️  Found {} wells, {} temp probes",
            headers.wells.len(),
            headers.temp_cols.iter().filter(|c| c.is_some()).count()
        );

        // Load well IDs for this experiment
        self.load_well_ids(&headers, experiment_id).await?;

        let mut temperature_readings_batch = Vec::new(); // 3. Prepare batch collections
        let mut phase_transitions_batch = Vec::new();

        for (row_idx, row) in rows.iter().skip(7).enumerate() {
            match self.parse_row_data(row, &headers, experiment_id, row_idx + 8) {
                Ok((temperature_reading, phase_transitions)) => {
                    if let Some(temp_reading) = temperature_reading {
                        temperature_readings_batch.push(temp_reading);
                    }
                    phase_transitions_batch.extend(phase_transitions);
                }
                Err(e) => {
                    errors.push(format!("Row {} error: {}", row_idx + 8, e));
                    if errors.len() > 10 {
                        break; // Stop if too many errors
                    }
                }
            }
        }

        let insert_start = std::time::Instant::now(); // Start batch import
        if !temperature_readings_batch.is_empty() {
            for chunk in temperature_readings_batch.chunks(CHUNK_SIZE) {
                temperature_readings::Entity::insert_many(chunk.to_vec())
                    .exec(&self.db)
                    .await
                    .context("Failed to batch insert temperature readings")?;
            }
        }

        if !phase_transitions_batch.is_empty() {
            println!(
                "   💾 Inserting {} phase transitions...",
                phase_transitions_batch.len()
            );

            for chunk in phase_transitions_batch.chunks(CHUNK_SIZE) {
                well_phase_transitions::Entity::insert_many(chunk.to_vec())
                    .exec(&self.db)
                    .await
                    .context("Failed to batch insert phase transitions")?;
            }
        }

        let insert_time = insert_start.elapsed().as_millis();
        let processing_time = start_time.elapsed().as_millis();

        println!("✅ Phase transition processing complete!");
        println!(
            "   🌡️  Temperature readings: {}",
            temperature_readings_batch.len()
        );
        println!("   🔄 Phase transitions: {}", phase_transitions_batch.len());
        println!("   🧪 Wells mapped: {}", headers.wells.len());
        println!("   ⏱️  Database insert time: {insert_time}ms");
        println!("   ⏱️  Total time: {processing_time}ms");

        Ok(DirectProcessingResult {
            success: errors.len() < 10,
            temperature_readings_created: temperature_readings_batch.len(),
            phase_transitions_created: phase_transitions_batch.len(),
            well_states_created: 0, // No longer storing all well states, only transitions
            time_points_created: 0, // No longer using time_points table
            wells_tracked: headers.wells.len(),
            errors,
            processing_time_ms: processing_time,
        })
    }

    fn parse_row_data(
        &mut self,
        row: &[Data],
        headers: &ColumnHeaders,
        experiment_id: Uuid,
        row_number: usize,
    ) -> Result<(
        Option<temperature_readings::ActiveModel>,
        Vec<well_phase_transitions::ActiveModel>,
    )> {
        // Extract timestamp
        let timestamp = self.extract_timestamp(row, headers, row_number)?;
        let parsed_timestamp = self.parse_timestamp_to_datetime(&timestamp)?;
        let timestamp_with_tz = Utc.from_utc_datetime(&parsed_timestamp).fixed_offset();

        // Extract image filename
        let image_filename = headers
            .image_col
            .and_then(|col| row.get(col))
            .and_then(|cell| match cell {
                Data::String(s) => Some(s.clone()),
                _ => None,
            });

        // Create temperature reading (store every timestamp for complete temperature profile)
        let mut temperature_reading = None;
        let mut has_temperatures = false;
        let mut probe_values = [None; 8];

        for (probe_idx, temp_col) in headers.temp_cols.iter().enumerate() {
            if let Some(col_idx) = temp_col {
                if let Some(temp) = row.get(*col_idx).and_then(|cell| match cell {
                    Data::Float(f) => Some(*f),
                    Data::Int(i) => Some(*i as f64),
                    _ => None,
                }) {
                    probe_values[probe_idx] =
                        Some(Decimal::from_f64_retain(temp).unwrap_or_default());
                    has_temperatures = true;
                }
            }
        }

        if has_temperatures {
            temperature_reading = Some(temperature_readings::ActiveModel {
                id: Set(Uuid::new_v4()),
                experiment_id: Set(experiment_id),
                timestamp: Set(timestamp_with_tz),
                image_filename: Set(image_filename),
                probe_1: Set(probe_values[0]),
                probe_2: Set(probe_values[1]),
                probe_3: Set(probe_values[2]),
                probe_4: Set(probe_values[3]),
                probe_5: Set(probe_values[4]),
                probe_6: Set(probe_values[5]),
                probe_7: Set(probe_values[6]),
                probe_8: Set(probe_values[7]),
                created_at: Set(Utc::now().fixed_offset()),
            });
        }

        // Track phase changes and create transitions only when state changes
        let mut phase_transitions = Vec::new();

        for well in &headers.wells {
            if let Some(current_phase) = row.get(well.col_idx).and_then(|cell| match cell {
                Data::Int(i) => Some(*i as i32),
                Data::Float(f) => Some(*f as i32),
                _ => None,
            }) {
                let well_key = format!("{}:{}", well.tray_name, well.well_coordinate);

                // Check if this well's phase state has changed
                let previous_phase = self.well_states.get(&well_key).copied();

                if let Some(prev) = previous_phase {
                    // Only create transition record if state actually changed
                    if prev != current_phase {
                        // We need to find the temperature_reading_id for this transition
                        // For now, we'll create a placeholder - in a real implementation
                        // we'd link to the actual temperature reading
                        let temp_reading_id = temperature_reading
                            .as_ref()
                            .map(|tr| match &tr.id {
                                Set(id) => *id,
                                _ => Uuid::new_v4(),
                            })
                            .unwrap_or_else(Uuid::new_v4);

                        // Get the actual well_id from our loaded well IDs
                        let well_id = *self
                            .well_ids
                            .get(&well_key)
                            .ok_or_else(|| anyhow!("Well ID not found for {}", well_key))?;

                        phase_transitions.push(well_phase_transitions::ActiveModel {
                            id: Set(Uuid::new_v4()),
                            well_id: Set(well_id),
                            experiment_id: Set(experiment_id),
                            temperature_reading_id: Set(temp_reading_id),
                            timestamp: Set(timestamp_with_tz),
                            previous_state: Set(prev),
                            new_state: Set(current_phase),
                            created_at: Set(Utc::now().fixed_offset()),
                        });
                    }
                } else {
                    // First time seeing this well - only create transition if it's not the default state (0)
                    if current_phase != 0 {
                        let temp_reading_id = temperature_reading
                            .as_ref()
                            .map(|tr| match &tr.id {
                                Set(id) => *id,
                                _ => Uuid::new_v4(),
                            })
                            .unwrap_or_else(Uuid::new_v4);

                        let well_id = *self
                            .well_ids
                            .get(&well_key)
                            .ok_or_else(|| anyhow!("Well ID not found for {}", well_key))?;

                        phase_transitions.push(well_phase_transitions::ActiveModel {
                            id: Set(Uuid::new_v4()),
                            well_id: Set(well_id),
                            experiment_id: Set(experiment_id),
                            temperature_reading_id: Set(temp_reading_id),
                            timestamp: Set(timestamp_with_tz),
                            previous_state: Set(0), // Assume starting state is liquid (0)
                            new_state: Set(current_phase),
                            created_at: Set(Utc::now().fixed_offset()),
                        });
                    }
                }

                // Update our tracking state
                self.well_states.insert(well_key, current_phase);
            }
        }

        Ok((temperature_reading, phase_transitions))
    }

    fn parse_headers(&self, rows: &[&[Data]]) -> Result<ColumnHeaders> {
        if rows.len() < 7 {
            return Err(anyhow!(
                "Excel file format invalid: insufficient header rows"
            ));
        }

        let tray_row = &rows[0]; // Row 1: tray names (P1, P1, P1..., P2, P2, P2...)
        let coordinate_row = &rows[1]; // Row 2: well coordinates (A1, A2, A3..., A1, A2, A3...)
        let header_row = &rows[6]; // Row 7: column headers (Date, Time, Temperature, (), (), ...)

        let mut headers = ColumnHeaders {
            date_col: None,
            time_col: None,
            image_col: None,
            temp_cols: [None; 8],
            wells: Vec::new(),
        };

        // Parse column headers
        for (col_idx, cell) in header_row.iter().enumerate() {
            if let Data::String(header) = cell {
                match header.as_str() {
                    "Date" => headers.date_col = Some(col_idx),
                    "Time" => headers.time_col = Some(col_idx),
                    "(.jpg)" => headers.image_col = Some(col_idx),
                    h if h.starts_with("Temperature") && h.contains('1') => {
                        headers.temp_cols[0] = Some(col_idx);
                    }
                    h if h.starts_with("Temperature") && h.contains('2') => {
                        headers.temp_cols[1] = Some(col_idx);
                    }
                    h if h.starts_with("Temperature") && h.contains('3') => {
                        headers.temp_cols[2] = Some(col_idx);
                    }
                    h if h.starts_with("Temperature") && h.contains('4') => {
                        headers.temp_cols[3] = Some(col_idx);
                    }
                    h if h.starts_with("Temperature") && h.contains('5') => {
                        headers.temp_cols[4] = Some(col_idx);
                    }
                    h if h.starts_with("Temperature") && h.contains('6') => {
                        headers.temp_cols[5] = Some(col_idx);
                    }
                    h if h.starts_with("Temperature") && h.contains('7') => {
                        headers.temp_cols[6] = Some(col_idx);
                    }
                    h if h.starts_with("Temperature") && h.contains('8') => {
                        headers.temp_cols[7] = Some(col_idx);
                    }
                    "()" => {
                        // This is a well state column - get tray and coordinate
                        if let (Some(tray_cell), Some(coord_cell)) =
                            (tray_row.get(col_idx), coordinate_row.get(col_idx))
                        {
                            if let (Data::String(tray_name), Data::String(well_coordinate)) =
                                (tray_cell, coord_cell)
                            {
                                if self.is_valid_tray_name(tray_name)
                                    && self.is_valid_well_coordinate(well_coordinate)
                                {
                                    // Convert well coordinate to (row, col) - A1 = (1,1), B2 = (2,2), etc.
                                    if let Ok((row, col)) =
                                        self.parse_well_coordinate(well_coordinate)
                                    {
                                        headers.wells.push(WellMapping {
                                            col_idx,
                                            row,
                                            col,
                                            tray_name: tray_name.clone(),
                                            well_coordinate: well_coordinate.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    _ => {} // Ignore unknown columns
                }
            }
        }

        if headers.wells.is_empty() {
            return Err(anyhow!("No valid wells found in headers"));
        }

        Ok(headers)
    }

    fn extract_timestamp(
        &self,
        row: &[Data],
        headers: &ColumnHeaders,
        row_number: usize,
    ) -> Result<String> {
        let date = headers
            .date_col
            .and_then(|col| row.get(col))
            .ok_or_else(|| anyhow!("Missing date column in row {}", row_number))?;

        let time = headers
            .time_col
            .and_then(|col| row.get(col))
            .ok_or_else(|| anyhow!("Missing time column in row {}", row_number))?;

        self.parse_datetime(date, time)
            .context(format!("Failed to parse datetime in row {row_number}"))
    }

    fn parse_datetime(&self, date_cell: &Data, time_cell: &Data) -> Result<String> {
        match (date_cell, time_cell) {
            (Data::String(date_str), Data::String(time_str)) => {
                let combined = format!("{date_str} {time_str}");
                match NaiveDateTime::parse_from_str(&combined, "%Y-%m-%d %H:%M:%S") {
                    Ok(dt) => Ok(dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string()),
                    Err(_) => match NaiveDateTime::parse_from_str(&combined, "%m/%d/%Y %H:%M:%S") {
                        Ok(dt) => Ok(dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string()),
                        Err(_) => Err(anyhow!("Could not parse datetime: {}", combined)),
                    },
                }
            }
            (Data::DateTime(excel_dt), _) => {
                // Handle Excel datetime format with millisecond precision
                let days = excel_dt.as_f64();
                let days_since_1900 = days.floor() as i64;
                let base = chrono::NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
                let date = base + chrono::Duration::days(days_since_1900 - 2); // Excel 1900 bug
                let time_fraction = days - days.floor();
                let total_seconds = time_fraction * 86400.0;
                let seconds = total_seconds.floor() as u32;
                let nanoseconds =
                    ((total_seconds - total_seconds.floor()) * 1_000_000_000.0) as u32;
                let time =
                    chrono::NaiveTime::from_num_seconds_from_midnight_opt(seconds, nanoseconds)
                        .unwrap_or_default();
                let datetime = chrono::NaiveDateTime::new(date, time);
                Ok(datetime.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
            }
            _ => Err(anyhow!("Unsupported datetime format")),
        }
    }

    fn parse_timestamp_to_datetime(&self, timestamp: &str) -> Result<NaiveDateTime> {
        // Try multiple formats
        if let Ok(dt) = NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S%.3f") {
            return Ok(dt);
        }
        if let Ok(dt) = NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S") {
            return Ok(dt);
        }
        if let Ok(dt) = NaiveDateTime::parse_from_str(timestamp, "%m/%d/%Y %H:%M:%S") {
            return Ok(dt);
        }
        Err(anyhow!("Could not parse timestamp: {}", timestamp))
    }

    fn parse_well_coordinate(&self, coordinate: &str) -> Result<(i32, i32)> {
        let well_coord = str_to_coordinates(coordinate)
            .map_err(|e| anyhow!("Invalid well coordinate '{}': {}", coordinate, e))?;

        // Convert u8 to i32 for consistency with existing code
        Ok((well_coord.row as i32, well_coord.column as i32))
    }

    fn is_valid_tray_name(&self, tray_name: &str) -> bool {
        tray_name == "P1" || tray_name == "P2"
    }

    fn is_valid_well_coordinate(&self, coordinate: &str) -> bool {
        // Use the primary coordinate parsing function for validation
        str_to_coordinates(coordinate).is_ok()
    }

    /// Load well IDs for all wells in the experiment
    async fn load_well_ids(&mut self, headers: &ColumnHeaders, experiment_id: Uuid) -> Result<()> {
        // First, get the experiment to find its tray configuration
        let experiment = experiments::Entity::find_by_id(experiment_id)
            .one(&self.db)
            .await
            .context("Failed to query experiment")?
            .ok_or_else(|| anyhow!("Experiment not found"))?;

        let tray_configuration_id = experiment
            .tray_configuration_id
            .ok_or_else(|| anyhow!("Experiment has no tray configuration"))?;

        println!(
            "   🔍 Loading wells for experiment {experiment_id} with tray config {tray_configuration_id}"
        );

        // Get all tray assignments for this configuration
        let tray_assignments: Vec<tray_configuration_assignments::Model> =
            tray_configuration_assignments::Entity::find()
                .filter(
                    tray_configuration_assignments::Column::TrayConfigurationId
                        .eq(tray_configuration_id),
                )
                .all(&self.db)
                .await
                .context("Failed to query tray assignments")?;

        // Get the tray IDs
        let tray_ids: Vec<Uuid> = tray_assignments
            .into_iter()
            .map(|assignment| assignment.tray_id)
            .collect();

        // Get all trays
        let all_trays: Vec<trays::Model> = trays::Entity::find()
            .filter(trays::Column::Id.is_in(tray_ids))
            .all(&self.db)
            .await
            .context("Failed to query trays")?;

        // Create a map of tray name -> tray_id
        let tray_name_to_id: HashMap<String, Uuid> = all_trays
            .into_iter()
            .filter_map(|tray| tray.name.map(|name| (name, tray.id)))
            .collect();

        println!(
            "   📋 Found {} trays: {:?}",
            tray_name_to_id.len(),
            tray_name_to_id.keys().collect::<Vec<_>>()
        );

        // Ensure wells exist for all trays, create if missing or recreate if dimensions don't match
        for (tray_name, &tray_id) in &tray_name_to_id {
            let existing_wells: Vec<wells::Model> = wells::Entity::find()
                .filter(wells::Column::TrayId.eq(tray_id))
                .all(&self.db)
                .await
                .context("Failed to query existing wells")?;

            // Check if we need to recreate wells based on Excel requirements
            let wells_for_tray: Vec<&WellMapping> = headers
                .wells
                .iter()
                .filter(|w| w.tray_name == *tray_name)
                .collect();

            let max_row = wells_for_tray.iter().map(|w| w.row).max().unwrap_or(0);
            let max_col = wells_for_tray.iter().map(|w| w.col).max().unwrap_or(0);

            let existing_max_row = existing_wells
                .iter()
                .map(|w| w.row_number)
                .max()
                .unwrap_or(0);
            let existing_max_col = existing_wells
                .iter()
                .map(|w| w.column_number)
                .max()
                .unwrap_or(0);

            if existing_wells.is_empty() {
                println!("   🔧 Creating wells for tray {tray_name} ({tray_id})");
                self.create_wells_from_excel_headers(tray_id, &wells_for_tray)
                    .await?;
            } else if max_row > existing_max_row || max_col > existing_max_col {
                println!(
                    "   🔄 Recreating wells for tray {tray_name} - Excel needs row {max_row} col {max_col}, but max existing is row {existing_max_row} col {existing_max_col}"
                );

                // Delete existing wells
                wells::Entity::delete_many()
                    .filter(wells::Column::TrayId.eq(tray_id))
                    .exec(&self.db)
                    .await
                    .context("Failed to delete existing wells")?;

                // Create new wells
                self.create_wells_from_excel_headers(tray_id, &wells_for_tray)
                    .await?;
            } else {
                println!(
                    "   ✅ Tray {} has {} wells (sufficient for Excel requirements)",
                    tray_name,
                    existing_wells.len()
                );
            }
        }

        // For each well in our headers, find the corresponding well_id
        for well_mapping in &headers.wells {
            if let Some(&tray_id) = tray_name_to_id.get(&well_mapping.tray_name) {
                // Query for the well with matching tray_id, row, and column
                if let Some(well) = wells::Entity::find()
                    .filter(wells::Column::TrayId.eq(tray_id))
                    .filter(wells::Column::RowNumber.eq(well_mapping.row))
                    .filter(wells::Column::ColumnNumber.eq(well_mapping.col))
                    .one(&self.db)
                    .await
                    .context("Failed to query well")?
                {
                    let well_key = format!(
                        "{}:{}",
                        well_mapping.tray_name, well_mapping.well_coordinate
                    );
                    self.well_ids.insert(well_key, well.id);
                } else {
                    // Debug: check what wells exist for this tray
                    let existing_wells: Vec<wells::Model> = wells::Entity::find()
                        .filter(wells::Column::TrayId.eq(tray_id))
                        .all(&self.db)
                        .await
                        .context("Failed to query existing wells")?;

                    println!(
                        "   ❌ Well not found: {} (row {}, col {}) in tray {}",
                        well_mapping.well_coordinate,
                        well_mapping.row,
                        well_mapping.col,
                        well_mapping.tray_name
                    );
                    println!(
                        "   🔍 Existing wells in tray {}: {} wells",
                        well_mapping.tray_name,
                        existing_wells.len()
                    );

                    if !existing_wells.is_empty() {
                        println!(
                            "   📍 Sample existing wells: {:?}",
                            existing_wells
                                .iter()
                                .take(5)
                                .map(|w| format!("row{},col{}", w.row_number, w.column_number))
                                .collect::<Vec<_>>()
                        );
                    }

                    return Err(anyhow!(
                        "Well not found: {} row {} col {} in tray {}. Found {} wells in tray.",
                        well_mapping.well_coordinate,
                        well_mapping.row,
                        well_mapping.col,
                        well_mapping.tray_name,
                        existing_wells.len()
                    ));
                }
            } else {
                return Err(anyhow!("Tray not found: {}", well_mapping.tray_name));
            }
        }

        println!("   🔗 Loaded {} well IDs", self.well_ids.len());
        Ok(())
    }

    /// Create wells based on what's actually found in the Excel headers
    async fn create_wells_from_excel_headers(
        &self,
        tray_id: Uuid,
        wells_for_tray: &[&WellMapping],
    ) -> Result<()> {
        let mut wells_data = Vec::new();

        for well_mapping in wells_for_tray {
            let well = wells::ActiveModel {
                id: Set(Uuid::new_v4()),
                tray_id: Set(tray_id),
                row_number: Set(well_mapping.row),
                column_number: Set(well_mapping.col),
                created_at: Set(Utc::now().into()),
                last_updated: Set(Utc::now().into()),
            };
            wells_data.push(well);
        }

        if !wells_data.is_empty() {
            // Batch insert wells
            wells::Entity::insert_many(wells_data)
                .exec(&self.db)
                .await
                .context("Failed to batch insert wells from Excel headers")?;

            println!(
                "   ✅ Created {} wells for tray from Excel headers",
                wells_for_tray.len()
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_well_coordinate() {
        let processor = DirectExcelProcessor::new(sea_orm::DatabaseConnection::Disconnected);

        // Test normal coordinates (row, column)
        assert_eq!(processor.parse_well_coordinate("A1").unwrap(), (1, 1));
        assert_eq!(processor.parse_well_coordinate("B2").unwrap(), (2, 2));
        assert_eq!(processor.parse_well_coordinate("H12").unwrap(), (12, 8));

        // Test invalid coordinates
        assert!(processor.parse_well_coordinate("1A").is_err());
        assert!(processor.parse_well_coordinate("").is_err());
        assert!(processor.parse_well_coordinate("a1").is_err()); // lowercase not supported by primary function
    }

    #[test]
    fn test_is_valid_tray_name() {
        let processor = DirectExcelProcessor::new(sea_orm::DatabaseConnection::Disconnected);

        assert!(processor.is_valid_tray_name("P1"));
        assert!(processor.is_valid_tray_name("P2"));
        assert!(!processor.is_valid_tray_name("P3"));
        assert!(!processor.is_valid_tray_name(""));
    }

    #[test]
    fn test_is_valid_well_coordinate() {
        let processor = DirectExcelProcessor::new(sea_orm::DatabaseConnection::Disconnected);

        assert!(processor.is_valid_well_coordinate("A1"));
        assert!(processor.is_valid_well_coordinate("H12"));
        assert!(processor.is_valid_well_coordinate("Z1")); // Z is valid (column 26)
        assert!(!processor.is_valid_well_coordinate("a1")); // lowercase not supported by primary function
        assert!(!processor.is_valid_well_coordinate(""));
        assert!(!processor.is_valid_well_coordinate("1A"));
    }
}
