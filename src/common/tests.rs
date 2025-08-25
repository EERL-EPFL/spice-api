use super::models::{ProcessingStatus, UIConfiguration, HealthCheck};

#[test]
fn test_processing_status_variants() {
    // Test that all ProcessingStatus variants exist and are serializable
    let pending = ProcessingStatus::Pending;
    let in_progress = ProcessingStatus::InProgress;
    let completed = ProcessingStatus::Completed;
    let failed = ProcessingStatus::Failed;

    // Test Debug trait
    assert_eq!(format!("{pending:?}"), "Pending");
    assert_eq!(format!("{in_progress:?}"), "InProgress");
    assert_eq!(format!("{completed:?}"), "Completed");
    assert_eq!(format!("{failed:?}"), "Failed");
}

#[test]
fn test_processing_status_serialization() {
    // Test serialization to JSON
    let pending = ProcessingStatus::Pending;
    let json = serde_json::to_string(&pending).unwrap();
    assert_eq!(json, r#""pending""#);

    let in_progress = ProcessingStatus::InProgress;
    let json = serde_json::to_string(&in_progress).unwrap();
    assert_eq!(json, r#""in_progress""#);

    let completed = ProcessingStatus::Completed;
    let json = serde_json::to_string(&completed).unwrap();
    assert_eq!(json, r#""completed""#);

    let failed = ProcessingStatus::Failed;
    let json = serde_json::to_string(&failed).unwrap();
    assert_eq!(json, r#""failed""#);
}

#[test]
fn test_processing_status_deserialization() {
    // Test deserialization from JSON
    let pending: ProcessingStatus = serde_json::from_str(r#""pending""#).unwrap();
    assert!(matches!(pending, ProcessingStatus::Pending));

    let in_progress: ProcessingStatus = serde_json::from_str(r#""in_progress""#).unwrap();
    assert!(matches!(in_progress, ProcessingStatus::InProgress));

    let completed: ProcessingStatus = serde_json::from_str(r#""completed""#).unwrap();
    assert!(matches!(completed, ProcessingStatus::Completed));

    let failed: ProcessingStatus = serde_json::from_str(r#""failed""#).unwrap();
    assert!(matches!(failed, ProcessingStatus::Failed));
}

#[test]
fn test_processing_status_clone_and_equality() {
    // Test Clone and PartialEq traits
    let status1 = ProcessingStatus::Pending;
    let status2 = status1.clone();
    assert_eq!(status1, status2);

    let status3 = ProcessingStatus::InProgress;
    assert_ne!(status1, status3);
}

#[test]
fn test_ui_configuration_default() {
    // Test Default trait implementation
    let config = UIConfiguration::default();
    assert_eq!(config.client_id, "");
    assert_eq!(config.realm, "");
    assert_eq!(config.url, "");
    assert_eq!(config.deployment, "");
}

#[test]
fn test_ui_configuration_creation() {
    // Test UIConfiguration can be created with specific values
    let config = UIConfiguration {
        client_id: "test-client".to_string(),
        realm: "test-realm".to_string(),
        url: "http://localhost:8080".to_string(),
        deployment: "test".to_string(),
    };
    
    // Test that the struct fields are accessible and correct
    assert_eq!(config.client_id, "test-client");
    assert_eq!(config.realm, "test-realm");
    assert_eq!(config.url, "http://localhost:8080");
    assert_eq!(config.deployment, "test");
}

#[test]
fn test_ui_configuration_serialization() {
    // Test serialization of UIConfiguration
    let config = UIConfiguration {
        client_id: "test-client".to_string(),
        realm: "test-realm".to_string(),
        url: "http://localhost:8080".to_string(),
        deployment: "test".to_string(),
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("test-client"));
    assert!(json.contains("clientId")); // Test snake_case -> camelCase conversion
    assert!(json.contains("test-realm"));
    assert!(json.contains("http://localhost:8080"));
    assert!(json.contains("test"));
}

#[test]
fn test_ui_configuration_deserialization() {
    // Test deserialization of UIConfiguration
    let json = r#"{"clientId":"test-client","realm":"test-realm","url":"http://localhost:8080","deployment":"test"}"#;
    let config: UIConfiguration = serde_json::from_str(json).unwrap();
    
    assert_eq!(config.client_id, "test-client");
    assert_eq!(config.realm, "test-realm");
    assert_eq!(config.url, "http://localhost:8080");
    assert_eq!(config.deployment, "test");
}

#[test]
fn test_health_check_serialization() {
    // Test HealthCheck serialization
    let health = HealthCheck {
        status: "ok".to_string(),
    };

    let json = serde_json::to_string(&health).unwrap();
    assert!(json.contains("ok"));
    assert!(json.contains("status"));
}

#[test]
fn test_health_check_deserialization() {
    // Test HealthCheck deserialization
    let json = r#"{"status":"ok"}"#;
    let health: HealthCheck = serde_json::from_str(json).unwrap();
    assert_eq!(health.status, "ok");
}

#[test]
fn test_processing_status_invalid_deserialization() {
    // Test that invalid processing status fails gracefully
    let result: Result<ProcessingStatus, _> = serde_json::from_str(r#""invalid_status""#);
    assert!(result.is_err());
}