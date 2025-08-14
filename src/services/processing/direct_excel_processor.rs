use crate::routes::tray_configurations::services::str_to_coordinates;
use crate::routes::{
    experiments::{
        models as experiments, phase_transitions::models as well_phase_transitions,
        temperatures::models as temperature_readings,
    },
    tray_configurations::{
        trays::models as tray_configuration_assignments, wells::models as wells,
    },
};
use anyhow::{Context, Result, anyhow};
use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};
use chrono::{NaiveDateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
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

        let headers = Self::parse_headers(&rows)?; // Identify tray and well headers from first two rows, then probe/time headers
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
            wells_tracked: headers.wells.len(),
            errors,
            processing_time_ms: processing_time,
        })
    }

    // Helper function to extract and process temperature readings
    fn create_temperature_reading(
        experiment_id: Uuid,
        timestamp_utc: chrono::DateTime<chrono::Utc>,
        probe_values: &[Option<Decimal>],
        image_filename: Option<String>,
    ) -> temperature_readings::ActiveModel {
        temperature_readings::ActiveModel {
            id: Set(Uuid::new_v4()),
            experiment_id: Set(experiment_id),
            timestamp: Set(timestamp_utc),
            probe_1: Set(probe_values[0]),
            probe_2: Set(probe_values[1]),
            probe_3: Set(probe_values[2]),
            probe_4: Set(probe_values[3]),
            probe_5: Set(probe_values[4]),
            probe_6: Set(probe_values[5]),
            probe_7: Set(probe_values[6]),
            probe_8: Set(probe_values[7]),
            image_filename: Set(image_filename),
            created_at: Set(chrono::Utc::now()),
        }
    }

    // Helper function to process phase transitions for wells
    fn process_well_phase_transitions(
        &mut self,
        row: &[Data],
        headers: &ColumnHeaders,
        experiment_id: Uuid,
        timestamp_utc: chrono::DateTime<chrono::Utc>,
        temperature_reading: Option<&temperature_readings::ActiveModel>,
    ) -> Result<Vec<well_phase_transitions::ActiveModel>> {
        let mut phase_transitions = Vec::new();

        for well in &headers.wells {
            if let Some(current_phase) = row.get(well.col_idx).and_then(|cell| match cell {
                Data::Int(i) => Some(i32::try_from(*i).unwrap_or(0)),
                #[allow(clippy::cast_possible_truncation)]
                Data::Float(f) => Some(f.round() as i32), // Convert phase state (0/1) to integer
                _ => None,
            }) {
                let well_key = format!("{}:{}", well.tray_name, well.well_coordinate);

                // Check if this well's phase state has changed
                let previous_phase = self.well_states.get(&well_key).copied();

                if let Some(prev) = previous_phase {
                    // Only create transition record if state actually changed
                    if prev != current_phase {
                        let temp_reading_id =
                            temperature_reading.map_or_else(Uuid::new_v4, |tr| match &tr.id {
                                Set(id) => *id,
                                _ => Uuid::new_v4(),
                            });

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
                            timestamp: Set(timestamp_utc),
                            previous_state: Set(prev),
                            new_state: Set(current_phase),
                            created_at: Set(Utc::now()),
                        });
                    }
                } else if current_phase != 0 {
                    // Only create transition if it's not the default state (0)
                    let temp_reading_id =
                        temperature_reading.map_or_else(Uuid::new_v4, |tr| match &tr.id {
                            Set(id) => *id,
                            _ => Uuid::new_v4(),
                        });

                    let well_id = *self
                        .well_ids
                        .get(&well_key)
                        .ok_or_else(|| anyhow!("Well ID not found for {}", well_key))?;

                    phase_transitions.push(well_phase_transitions::ActiveModel {
                        id: Set(Uuid::new_v4()),
                        well_id: Set(well_id),
                        experiment_id: Set(experiment_id),
                        temperature_reading_id: Set(temp_reading_id),
                        timestamp: Set(timestamp_utc),
                        previous_state: Set(0), // Assume starting state is liquid (0)
                        new_state: Set(current_phase),
                        created_at: Set(Utc::now()),
                    });
                }
                self.well_states.insert(well_key, current_phase);
            }
        }

        Ok(phase_transitions)
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
        let timestamp = Self::extract_timestamp(row, headers, row_number)?; // Extract timestamp
        let parsed_timestamp = Self::parse_timestamp_to_datetime(&timestamp)?;
        let timestamp_utc = Utc.from_utc_datetime(&parsed_timestamp);

        // Extract image filename
        let image_filename = headers
            .image_col
            .and_then(|col| row.get(col))
            .and_then(|cell| match cell {
                Data::String(s) => Some(s.clone()),
                _ => None,
            });

        // Extract temperature readings
        let mut probe_values = [None; 8];
        let mut has_temperatures = false;

        for (probe_idx, temp_col) in headers.temp_cols.iter().enumerate() {
            if let Some(col_idx) = temp_col {
                if let Some(temp) = row.get(*col_idx).and_then(|cell| match cell {
                    Data::Float(f) => Some(*f),
                    #[allow(clippy::cast_precision_loss)]
                    Data::Int(i) => Some(*i as f64), // Intentional precision loss for temperature conversion
                    _ => None,
                }) {
                    probe_values[probe_idx] =
                        Some(Decimal::from_f64_retain(temp).unwrap_or_default());
                    has_temperatures = true;
                }
            }
        }

        // Create temperature reading if we have temperature data
        let temperature_reading = if has_temperatures {
            Some(Self::create_temperature_reading(
                experiment_id,
                timestamp_utc,
                &probe_values,
                image_filename,
            ))
        } else {
            None
        };

        // Process phase transitions for all wells
        let phase_transitions = self.process_well_phase_transitions(
            row,
            headers,
            experiment_id,
            timestamp_utc,
            temperature_reading.as_ref(),
        )?;

        Ok((temperature_reading, phase_transitions))
    }

    fn parse_headers(rows: &[&[Data]]) -> Result<ColumnHeaders> {
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
                                if Self::is_valid_tray_name(tray_name)
                                    && Self::is_valid_well_coordinate(well_coordinate)
                                {
                                    // Convert well coordinate to (row, col) - A1 = (1,1), B2 = (2,2), etc.
                                    if let Ok((row, col)) =
                                        Self::parse_well_coordinate(well_coordinate)
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

        Self::parse_datetime(date, time)
            .context(format!("Failed to parse datetime in row {row_number}"))
    }

    fn parse_datetime(date_cell: &Data, time_cell: &Data) -> Result<String> {
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
                #[allow(clippy::cast_possible_truncation)]
                let days_since_1900 = days.floor().round() as i64;
                let base = chrono::NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
                let date = base + chrono::Duration::days(days_since_1900 - 2); // Excel 1900 bug
                let time_fraction = days - days.floor();
                let total_seconds = time_fraction * 86400.0;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let seconds = total_seconds.floor().max(0.0) as u32;
                let nanoseconds = {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        ((total_seconds - total_seconds.floor()) * 1_000_000_000.0).max(0.0) as u32
                    }
                };
                let time =
                    chrono::NaiveTime::from_num_seconds_from_midnight_opt(seconds, nanoseconds)
                        .unwrap_or_default();
                let datetime = chrono::NaiveDateTime::new(date, time);
                Ok(datetime.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
            }
            _ => Err(anyhow!("Unsupported datetime format")),
        }
    }

    fn parse_timestamp_to_datetime(timestamp: &str) -> Result<NaiveDateTime> {
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

    fn parse_well_coordinate(coordinate: &str) -> Result<(i32, i32)> {
        let well_coord = str_to_coordinates(coordinate)
            .map_err(|e| anyhow!("Invalid well coordinate '{}': {}", coordinate, e))?;

        // WellCoordinate now correctly maps: row=letter(A=1,B=2), column=number(1,2,12)
        // Return (row, column) for database storage
        // For H12: WellCoordinate{row=8, column=12} -> return (8, 12)
        Ok((i32::from(well_coord.row), i32::from(well_coord.column)))
    }

    fn is_valid_tray_name(tray_name: &str) -> bool {
        tray_name == "P1" || tray_name == "P2"
    }

    fn is_valid_well_coordinate(coordinate: &str) -> bool {
        // Use the primary coordinate parsing function for validation
        str_to_coordinates(coordinate).is_ok()
    }

    /// Load well IDs for all wells in the experiment
    async fn load_well_ids(&mut self, headers: &ColumnHeaders, experiment_id: Uuid) -> Result<()> {
        let tray_configuration_id = self.get_experiment_tray_config(experiment_id).await?;
        let tray_name_to_id = self.load_tray_mapping(tray_configuration_id).await?;
        self.ensure_wells_exist(headers, &tray_name_to_id).await?;
        self.map_well_ids_from_headers(headers, &tray_name_to_id)
            .await?;

        println!("   🔗 Loaded {} well IDs", self.well_ids.len());
        Ok(())
    }

    /// Get tray configuration ID for the experiment
    async fn get_experiment_tray_config(&self, experiment_id: Uuid) -> Result<Uuid> {
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

        Ok(tray_configuration_id)
    }

    /// Load mapping of tray names to tray IDs
    async fn load_tray_mapping(
        &self,
        tray_configuration_id: Uuid,
    ) -> Result<HashMap<String, Uuid>> {
        let tray_assignments: Vec<tray_configuration_assignments::Model> =
            tray_configuration_assignments::Entity::find()
                .filter(
                    tray_configuration_assignments::Column::TrayConfigurationId
                        .eq(tray_configuration_id),
                )
                .all(&self.db)
                .await
                .context("Failed to query tray assignments")?;

        // After schema simplification, all tray data is embedded in assignments
        // So we can build the name-to-id mapping directly from assignments
        let tray_name_to_id: HashMap<String, Uuid> = tray_assignments
            .into_iter()
            .filter_map(|assignment| assignment.name.map(|name| (name, assignment.id)))
            .collect();

        println!(
            "   📋 Found {} trays: {:?}",
            tray_name_to_id.len(),
            tray_name_to_id.keys().collect::<Vec<_>>()
        );

        Ok(tray_name_to_id)
    }

    /// Ensure wells exist for all trays, create or recreate as needed
    async fn ensure_wells_exist(
        &self,
        headers: &ColumnHeaders,
        tray_name_to_id: &HashMap<String, Uuid>,
    ) -> Result<()> {
        for (tray_name, &tray_id) in tray_name_to_id {
            self.ensure_tray_wells_exist(headers, tray_name, tray_id)
                .await?;
        }
        Ok(())
    }

    /// Ensure wells exist for a specific tray
    async fn ensure_tray_wells_exist(
        &self,
        headers: &ColumnHeaders,
        tray_name: &str,
        tray_id: Uuid,
    ) -> Result<()> {
        let existing_wells: Vec<wells::Model> = wells::Entity::find()
            .filter(wells::Column::TrayId.eq(tray_id))
            .all(&self.db)
            .await
            .context("Failed to query existing wells")?;

        let wells_for_tray: Vec<&WellMapping> = headers
            .wells
            .iter()
            .filter(|w| w.tray_name == tray_name)
            .collect();

        let (max_row, max_col) = Self::get_required_dimensions(&wells_for_tray);
        let (existing_max_row, existing_max_col) = Self::get_existing_dimensions(&existing_wells);

        if existing_wells.is_empty() {
            println!("   🔧 Creating wells for tray {tray_name} ({tray_id})");
            self.create_wells_from_excel_headers(tray_id, &wells_for_tray)
                .await?;
        } else if max_row > existing_max_row || max_col > existing_max_col {
            println!(
                "   🔄 Recreating wells for tray {tray_name} - Excel needs row {max_row} col {max_col}, but max existing is row {existing_max_row} col {existing_max_col}"
            );
            self.recreate_wells(tray_id, &wells_for_tray).await?;
        } else {
            println!(
                "   ✅ Tray {} has {} wells (sufficient for Excel requirements)",
                tray_name,
                existing_wells.len()
            );
        }

        Ok(())
    }

    /// Get required dimensions from Excel well mappings
    fn get_required_dimensions(wells_for_tray: &[&WellMapping]) -> (i32, i32) {
        let max_row = wells_for_tray.iter().map(|w| w.row).max().unwrap_or(0);
        let max_col = wells_for_tray.iter().map(|w| w.col).max().unwrap_or(0);
        (max_row, max_col)
    }

    /// Get existing dimensions from database wells
    fn get_existing_dimensions(existing_wells: &[wells::Model]) -> (i32, i32) {
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
        (existing_max_row, existing_max_col)
    }

    /// Recreate wells by deleting existing ones and creating new ones
    async fn recreate_wells(&self, tray_id: Uuid, wells_for_tray: &[&WellMapping]) -> Result<()> {
        wells::Entity::delete_many()
            .filter(wells::Column::TrayId.eq(tray_id))
            .exec(&self.db)
            .await
            .context("Failed to delete existing wells")?;

        self.create_wells_from_excel_headers(tray_id, wells_for_tray)
            .await
    }

    /// Map well IDs from headers to internal `well_ids` `HashMap`
    async fn map_well_ids_from_headers(
        &mut self,
        headers: &ColumnHeaders,
        tray_name_to_id: &HashMap<String, Uuid>,
    ) -> Result<()> {
        for well_mapping in &headers.wells {
            if let Some(&tray_id) = tray_name_to_id.get(&well_mapping.tray_name) {
                self.find_and_store_well_id(well_mapping, tray_id).await?;
            } else {
                return Err(anyhow!("Tray not found: {}", well_mapping.tray_name));
            }
        }
        Ok(())
    }

    /// Find and store a single well ID
    async fn find_and_store_well_id(
        &mut self,
        well_mapping: &WellMapping,
        tray_id: Uuid,
    ) -> Result<()> {
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
            Ok(())
        } else {
            self.handle_well_not_found_error(well_mapping, tray_id)
                .await
        }
    }

    /// Handle error when well is not found
    async fn handle_well_not_found_error(
        &self,
        well_mapping: &WellMapping,
        tray_id: Uuid,
    ) -> Result<()> {
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

        Err(anyhow!(
            "Well not found: {} row {} col {} in tray {}. Found {} wells in tray.",
            well_mapping.well_coordinate,
            well_mapping.row,
            well_mapping.col,
            well_mapping.tray_name,
            existing_wells.len()
        ))
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
                created_at: Set(Utc::now()),
                last_updated: Set(Utc::now()),
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
        let _processor = DirectExcelProcessor::new(sea_orm::DatabaseConnection::Disconnected);

        // Test coordinates with corrected WellCoordinate mapping
        // A1 = row 1 (A), column 1 (1)
        assert_eq!(
            DirectExcelProcessor::parse_well_coordinate("A1").unwrap(),
            (1, 1)
        );
        // B2 = row 2 (B), column 2 (2)
        assert_eq!(
            DirectExcelProcessor::parse_well_coordinate("B2").unwrap(),
            (2, 2)
        );
        // H12 = row 8 (H), column 12 (12)
        assert_eq!(
            DirectExcelProcessor::parse_well_coordinate("H12").unwrap(),
            (8, 12)
        );

        // Test invalid coordinates
        assert!(DirectExcelProcessor::parse_well_coordinate("1A").is_err());
        assert!(DirectExcelProcessor::parse_well_coordinate("").is_err());
        assert!(DirectExcelProcessor::parse_well_coordinate("a1").is_err()); // lowercase not supported by primary function
    }

    #[test]
    fn test_is_valid_tray_name() {
        let _processor = DirectExcelProcessor::new(sea_orm::DatabaseConnection::Disconnected);

        assert!(DirectExcelProcessor::is_valid_tray_name("P1"));
        assert!(DirectExcelProcessor::is_valid_tray_name("P2"));
        assert!(!DirectExcelProcessor::is_valid_tray_name("P3"));
        assert!(!DirectExcelProcessor::is_valid_tray_name(""));
    }

    #[test]
    fn test_is_valid_well_coordinate() {
        let _processor = DirectExcelProcessor::new(sea_orm::DatabaseConnection::Disconnected);

        assert!(DirectExcelProcessor::is_valid_well_coordinate("A1"));
        assert!(DirectExcelProcessor::is_valid_well_coordinate("H12"));
        assert!(DirectExcelProcessor::is_valid_well_coordinate("Z1")); // Z is valid (column 26)
        assert!(!DirectExcelProcessor::is_valid_well_coordinate("a1")); // lowercase not supported by primary function
        assert!(!DirectExcelProcessor::is_valid_well_coordinate(""));
        assert!(!DirectExcelProcessor::is_valid_well_coordinate("1A"));
    }
}
