use super::models::{NucleationEvent, NucleationStatistics};
use rust_decimal::Decimal;
use uuid::Uuid;

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
    assert!((stats.success_rate - 2.0 / 3.0).abs() < f64::EPSILON);
    assert!(stats.mean_nucleation_temp_celsius.is_some());
    assert!((stats.mean_nucleation_temp_celsius.unwrap() - (-16.5)).abs() < f64::EPSILON);
    assert_eq!(stats.median_nucleation_time_seconds, Some(1500)); // (1000 + 2000) / 2
}

#[test]
fn test_nucleation_statistics_empty() {
    let events = vec![];
    let stats = NucleationStatistics::from_events(&events);

    // Empty events should return None
    assert!(stats.is_none(), "Empty events should return None");
}