//! Database operations for Excel processing
//!
//! This module handles all database interactions needed during Excel processing,
//! including loading mappings, creating wells, and batch operations.

use crate::{
    experiments::models as experiments,
    experiments::{
        phase_transitions::models as phase_transitions,
        probe_temperature_readings::models as probe_temperature_readings,
        temperatures::models as temperature_readings,
    },
    tray_configurations::{
        probes::models as probes, trays::models as tray_configuration_assignments,
        wells::models as wells,
    },
};
use anyhow::{Context, Result, anyhow};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use uuid::Uuid;

use super::structure::{ExcelStructure, parse_well_coordinate};

/// Database operations for Excel processing
pub struct DatabaseOperations {
    pub db: DatabaseConnection,
}

impl DatabaseOperations {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Load well mappings from database for the given experiment
    pub async fn load_well_mappings(
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
                    if let Ok((row_letter, column_number)) = parse_well_coordinate(well_coord) {
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

    /// Load probe mappings from database for the given experiment
    pub async fn load_probe_mappings(&self, experiment_id: Uuid) -> Result<HashMap<usize, Uuid>> {
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
                // Excel structure: Date(0), Time(1), Temp1(2), Temp2(3), ..., Temp8(9)
                // But data_column_index is stored as 1-8 (user-friendly), so we need to add 1
                #[allow(clippy::cast_sign_loss)]
                // data_column_index is always positive in this context
                let col_index = (probe.data_column_index + 1) as usize;
                probe_mappings.insert(col_index, probe.id);
            }
        }

        tracing::debug!(
            "Loaded {} probe mappings from database",
            probe_mappings.len()
        );
        Ok(probe_mappings)
    }

    /// Load tray mappings from database for the given experiment
    pub async fn load_tray_mappings(&self, experiment_id: Uuid) -> Result<HashMap<String, Uuid>> {
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

    /// Ensure wells exist for all trays mentioned in the Excel structure
    pub async fn ensure_wells_exist(
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

    /// Ensure wells exist for a specific tray
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

    /// Create wells from Excel headers
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
}

/// Batch container for database operations
#[derive(Debug, Default)]
pub struct ProcessingBatches {
    pub temp_readings: Vec<temperature_readings::ActiveModel>,
    pub probe_readings: Vec<probe_temperature_readings::ActiveModel>,
    pub phase_transitions: Vec<phase_transitions::ActiveModel>,
    pub temp_readings_total: usize,
    pub probe_readings_total: usize,
    pub phase_transitions_total: usize,
}

impl ProcessingBatches {
    pub fn total_count(&self) -> usize {
        self.temp_readings.len() + self.probe_readings.len() + self.phase_transitions.len()
    }

    /// Flush all batches to the database
    pub async fn flush(&mut self, db: &DatabaseConnection) -> Result<()> {
        // Update totals before draining
        self.temp_readings_total += self.temp_readings.len();
        self.probe_readings_total += self.probe_readings.len();
        self.phase_transitions_total += self.phase_transitions.len();

        // Insert batches
        if !self.temp_readings.is_empty() {
            temperature_readings::Entity::insert_many(self.temp_readings.drain(..))
                .exec(db)
                .await?;
        }
        if !self.probe_readings.is_empty() {
            probe_temperature_readings::Entity::insert_many(self.probe_readings.drain(..))
                .exec(db)
                .await?;
        }
        if !self.phase_transitions.is_empty() {
            phase_transitions::Entity::insert_many(self.phase_transitions.drain(..))
                .exec(db)
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_column_offset() {
        // Test that the probe column mapping correctly handles the Excel column offset
        // Excel structure: Date(0), Time(1), Temp1(2), Temp2(3), ..., Temp8(9)
        // Database stores: data_column_index 1-8 (user-friendly)
        // Mapping should convert: 1->2, 2->3, 3->4, 4->5, 5->6, 6->7, 7->8, 8->9
        
        struct MockProbe {
            data_column_index: i32,
            id: Uuid,
        }
        
        let mock_probes = vec![
            MockProbe { data_column_index: 1, id: Uuid::new_v4() },
            MockProbe { data_column_index: 2, id: Uuid::new_v4() },
            MockProbe { data_column_index: 3, id: Uuid::new_v4() },
            MockProbe { data_column_index: 4, id: Uuid::new_v4() },
            MockProbe { data_column_index: 5, id: Uuid::new_v4() },
            MockProbe { data_column_index: 6, id: Uuid::new_v4() },
            MockProbe { data_column_index: 7, id: Uuid::new_v4() },
            MockProbe { data_column_index: 8, id: Uuid::new_v4() },
        ];
        
        let mut probe_mappings = HashMap::new();
        for probe in &mock_probes {
            // Apply the same logic as in load_probe_mappings
            #[allow(clippy::cast_sign_loss)]
            let col_index = (probe.data_column_index + 1) as usize;
            probe_mappings.insert(col_index, probe.id);
        }
        
        // Verify mappings
        assert_eq!(probe_mappings.len(), 8);
        assert!(probe_mappings.contains_key(&2));  // Probe 1 -> Excel column 2
        assert!(probe_mappings.contains_key(&3));  // Probe 2 -> Excel column 3
        assert!(probe_mappings.contains_key(&4));  // Probe 3 -> Excel column 4
        assert!(probe_mappings.contains_key(&5));  // Probe 4 -> Excel column 5
        assert!(probe_mappings.contains_key(&6));  // Probe 5 -> Excel column 6
        assert!(probe_mappings.contains_key(&7));  // Probe 6 -> Excel column 7
        assert!(probe_mappings.contains_key(&8));  // Probe 7 -> Excel column 8
        assert!(probe_mappings.contains_key(&9));  // Probe 8 -> Excel column 9
        
        // Verify incorrect mappings don't exist
        assert!(!probe_mappings.contains_key(&0)); // No probe at column 0 (Date)
        assert!(!probe_mappings.contains_key(&1)); // No probe at column 1 (Time)
        assert!(!probe_mappings.contains_key(&10)); // No probe beyond column 9
    }
}
