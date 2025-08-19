use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Shared struct for nucleation events across experiments, samples, and treatments
/// Represents the scientific result of ice nucleation for a single well
/// Uses scientific naming conventions with explicit units
#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct NucleationEvent {
    /// Unique identifier for the experiment this event occurred in
    pub experiment_id: Uuid,
    /// Human-readable name of the experiment
    pub experiment_name: String,
    /// Date and time when the experiment was performed
    pub experiment_date: Option<DateTime<Utc>>,
    /// Well coordinate in standard format (e.g., "A1", "B2", "H12")
    pub well_coordinate: String,
    /// Name of the tray/plate (e.g., "P1", "P2")
    pub tray_name: Option<String>,
    /// Time from experiment start to nucleation in seconds
    pub nucleation_time_seconds: Option<i64>,
    /// Average temperature across all temperature probes at nucleation event, in Celsius
    pub nucleation_temperature_avg_celsius: Option<Decimal>,
    /// UI compatibility field - same as `nucleation_time_seconds`
    pub freezing_time_seconds: Option<i64>,
    /// UI compatibility field - same as `nucleation_temperature_avg_celsius`
    pub freezing_temperature_avg: Option<Decimal>,
    /// Dilution factor applied to the sample in this well
    pub dilution_factor: Option<i32>,
    /// Final state of the well: "frozen", "liquid", or "`no_data`"
    pub final_state: String,
    /// ID of the treatment applied to this sample
    pub treatment_id: Option<Uuid>,
    /// Name of the treatment applied to this sample
    pub treatment_name: Option<String>,
}

/// Summary statistics for nucleation events, used for sample and treatment analysis
#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct NucleationStatistics {
    /// Total number of wells tested
    pub total_wells: usize,
    /// Number of wells that nucleated (froze)
    pub frozen_count: usize,
    /// Number of wells that remained liquid
    pub liquid_count: usize,
    /// Success rate as a fraction (0.0 to 1.0)
    pub success_rate: f64,
    /// Mean nucleation temperature in Celsius for wells that froze
    pub mean_nucleation_temp_celsius: Option<f64>,
    /// Median nucleation time in seconds for wells that froze
    pub median_nucleation_time_seconds: Option<i64>,
}

/// Summary statistics grouped by dilution factor
#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DilutionSummary {
    /// The dilution factor for this group
    pub dilution_factor: i32,
    /// Statistics for wells at this dilution level
    pub statistics: NucleationStatistics,
}

impl NucleationStatistics {
    /// Calculate statistics from a collection of nucleation events
    pub fn from_events(events: &[NucleationEvent]) -> Option<Self> {
        if events.is_empty() {
            return None;
        }
        
        let total_wells = events.len();
        let frozen_events: Vec<_> = events
            .iter()
            .filter(|e| e.final_state == "frozen")
            .collect();
        let frozen_count = frozen_events.len();
        let liquid_count = events.iter().filter(|e| e.final_state == "liquid").count();

        let success_rate = if total_wells > 0 {
            frozen_count as f64 / total_wells as f64
        } else {
            0.0
        };

        // Calculate mean temperature for frozen wells
        let mean_nucleation_temp_celsius = if frozen_count > 0 {
            let temp_sum: f64 = frozen_events
                .iter()
                .filter_map(|e| e.nucleation_temperature_avg_celsius)
                .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0))
                .sum();
            Some(temp_sum / frozen_count as f64)
        } else {
            None
        };

        // Calculate median nucleation time for frozen wells
        let median_nucleation_time_seconds = if frozen_count > 0 {
            let mut times: Vec<i64> = frozen_events
                .iter()
                .filter_map(|e| e.nucleation_time_seconds)
                .collect();
            times.sort_unstable();

            if times.is_empty() {
                None
            } else if times.len() % 2 == 0 {
                let mid = times.len() / 2;
                Some((times[mid - 1] + times[mid]) / 2)
            } else {
                Some(times[times.len() / 2])
            }
        } else {
            None
        };

        Some(Self {
            total_wells,
            frozen_count,
            liquid_count,
            success_rate,
            mean_nucleation_temp_celsius,
            median_nucleation_time_seconds,
        })
    }

    /// Calculate dilution summaries from a collection of nucleation events
    pub fn dilution_summaries_from_events(events: &[NucleationEvent]) -> Vec<DilutionSummary> {
        use std::collections::HashMap;

        // Group events by dilution factor
        let mut dilution_groups: HashMap<i32, Vec<&NucleationEvent>> = HashMap::new();

        for event in events {
            let dilution = event.dilution_factor.unwrap_or(1);
            dilution_groups.entry(dilution).or_default().push(event);
        }

        // Calculate statistics for each dilution group
        let mut summaries: Vec<DilutionSummary> = dilution_groups
            .into_iter()
            .map(|(dilution_factor, group_events)| {
                // Convert references back to owned for statistics calculation
                let owned_events: Vec<NucleationEvent> =
                    group_events.into_iter().cloned().collect();
                let statistics = NucleationStatistics::from_events(&owned_events)
                    .expect("from_events should always return Some");

                DilutionSummary {
                    dilution_factor,
                    statistics,
                }
            })
            .collect();

        // Sort by dilution factor
        summaries.sort_by_key(|s| s.dilution_factor);
        summaries
    }
}

impl Eq for NucleationStatistics {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nucleation_statistics_calculation() {
        let events = vec![
            NucleationEvent {
                experiment_id: Uuid::new_v4(),
                experiment_name: "Test".to_string(),
                experiment_date: None,
                well_coordinate: "A1".to_string(),
                tray_name: Some("P1".to_string()),
                nucleation_time_seconds: Some(1000),
                nucleation_temperature_avg_celsius: Some(Decimal::new(-150, 1)), // -15.0
                freezing_time_seconds: Some(1000),                               // UI compatibility
                freezing_temperature_avg: Some(Decimal::new(-150, 1)),           // UI compatibility
                dilution_factor: Some(100),
                final_state: "frozen".to_string(),
                treatment_id: None,
                treatment_name: None,
            },
            NucleationEvent {
                experiment_id: Uuid::new_v4(),
                experiment_name: "Test".to_string(),
                experiment_date: None,
                well_coordinate: "A2".to_string(),
                tray_name: Some("P1".to_string()),
                nucleation_time_seconds: Some(2000),
                nucleation_temperature_avg_celsius: Some(Decimal::new(-180, 1)), // -18.0
                freezing_time_seconds: Some(2000),                               // UI compatibility
                freezing_temperature_avg: Some(Decimal::new(-180, 1)),           // UI compatibility
                dilution_factor: Some(100),
                final_state: "frozen".to_string(),
                treatment_id: None,
                treatment_name: None,
            },
            NucleationEvent {
                experiment_id: Uuid::new_v4(),
                experiment_name: "Test".to_string(),
                experiment_date: None,
                well_coordinate: "A3".to_string(),
                tray_name: Some("P1".to_string()),
                nucleation_time_seconds: None,
                nucleation_temperature_avg_celsius: None,
                freezing_time_seconds: None,    // UI compatibility
                freezing_temperature_avg: None, // UI compatibility
                dilution_factor: Some(100),
                final_state: "liquid".to_string(),
                treatment_id: None,
                treatment_name: None,
            },
        ];

        let stats = NucleationStatistics::from_events(&events).unwrap();

        assert_eq!(stats.total_wells, 3);
        assert_eq!(stats.frozen_count, 2);
        assert_eq!(stats.liquid_count, 1);
        assert_eq!(stats.success_rate, 2.0 / 3.0);
        assert!(stats.mean_nucleation_temp_celsius.is_some());
        assert_eq!(stats.mean_nucleation_temp_celsius.unwrap(), -16.5);
        assert_eq!(stats.median_nucleation_time_seconds, Some(1500)); // (1000 + 2000) / 2
    }

    #[test]
    fn test_nucleation_statistics_empty() {
        let events = vec![];
        let stats = NucleationStatistics::from_events(&events);

        // Empty events should return None
        assert!(stats.is_none(), "Empty events should return None");
    }
}
