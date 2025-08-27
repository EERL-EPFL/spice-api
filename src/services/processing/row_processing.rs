//! Row-by-row data processing logic for Excel files
//!
//! This module handles the row-by-row processing of Excel data, including
//! temperature readings, probe readings, and phase transitions.

use crate::experiments::{
    phase_transitions::models as phase_transitions,
    probe_temperature_readings::models as probe_temperature_readings,
    temperatures::models as temperature_readings,
};
use anyhow::Result;
use calamine::Data;
use chrono::{Timelike, Utc};
use sea_orm::Set;
use std::collections::HashMap;
use uuid::Uuid;

use super::{
    structure::ExcelStructure,
    utils::{extract_decimal, extract_image_filename, extract_integer, parse_timestamp},
};

/// Process a single row of Excel data
pub fn process_row(
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
    let timestamp = parse_timestamp(row, structure)?;
    let timestamp_clean = timestamp.with_nanosecond(0).unwrap_or(timestamp);

    // Create temperature reading
    let temp_reading = temperature_readings::ActiveModel {
        id: Set(Uuid::new_v4()),
        experiment_id: Set(experiment_id),
        timestamp: Set(timestamp_clean),
        image_filename: Set(extract_image_filename(row, structure)),
        created_at: Set(Utc::now()),
    };

    // Create probe readings
    let mut probe_readings = Vec::new();
    for &probe_col in &structure.probe_columns {
        if let (Some(cell), Some(&probe_id)) = (row.get(probe_col), probe_mappings.get(&probe_col))
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

/// Result of Excel file processing
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

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::Data;
    use std::collections::HashMap;

    #[test]
    fn test_process_row() {
        let mut structure = ExcelStructure {
            date_col: 0,
            time_col: 1,
            image_col: Some(2),
            well_columns: HashMap::new(),
            probe_columns: vec![3],
            data_start_row: 7,
        };

        structure.well_columns.insert("P1:A1".to_string(), 4);

        let mut well_mappings = HashMap::new();
        well_mappings.insert("P1:A1".to_string(), Uuid::new_v4());

        let mut probe_mappings = HashMap::new();
        probe_mappings.insert(3, Uuid::new_v4());

        let mut phase_states = HashMap::new();

        let row = vec![
            Data::String("2023-01-01".to_string()),
            Data::String("12:00:00".to_string()),
            Data::String("image.jpg".to_string()),
            Data::Float(-10.5),
            Data::Int(1),
        ];

        let result = process_row(
            &row,
            &structure,
            Uuid::new_v4(),
            &well_mappings,
            &probe_mappings,
            &mut phase_states,
        );

        assert!(result.is_ok());
        let (temp_reading, probe_readings, transitions) = result.unwrap();

        assert!(temp_reading.is_some());
        assert_eq!(probe_readings.len(), 1);
        assert_eq!(transitions.len(), 1); // Phase state changed from 0 to 1
    }
}
