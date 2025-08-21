use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use uuid::Uuid;
use crate::tray_configurations::probes::models::{Entity as ProbeEntity, Column as ProbeColumn};

#[derive(Debug)]
pub struct ProbeValidationError {
    pub message: String,
    pub field: String,
}

impl std::fmt::Display for ProbeValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ProbeValidationError {}

pub async fn validate_probe_uniqueness(
    db: &DatabaseConnection,
    tray_id: Uuid,
    name: &str,
    data_column_index: i32,
    probe_id: Option<Uuid>, // None for create, Some(id) for update
) -> Result<(), ProbeValidationError> {
    // Check for duplicate name within the same tray
    let mut name_query = ProbeEntity::find()
        .filter(ProbeColumn::TrayId.eq(tray_id))
        .filter(ProbeColumn::Name.eq(name));
    
    // For updates, exclude the current probe from the check
    if let Some(id) = probe_id {
        name_query = name_query.filter(ProbeColumn::Id.ne(id));
    }
    
    if let Some(_existing) = name_query.one(db).await.map_err(|_| ProbeValidationError {
        message: "Database error while checking for duplicate names".to_string(),
        field: "name".to_string(),
    })? {
        return Err(ProbeValidationError {
            message: "A probe with this name already exists on this tray".to_string(),
            field: "name".to_string(),
        });
    }

    // Check for duplicate data_column_index within the same tray
    let mut column_query = ProbeEntity::find()
        .filter(ProbeColumn::TrayId.eq(tray_id))
        .filter(ProbeColumn::DataColumnIndex.eq(data_column_index));
    
    // For updates, exclude the current probe from the check
    if let Some(id) = probe_id {
        column_query = column_query.filter(ProbeColumn::Id.ne(id));
    }
    
    if let Some(_existing) = column_query.one(db).await.map_err(|_| ProbeValidationError {
        message: "Database error while checking for duplicate column indices".to_string(),
        field: "data_column_index".to_string(),
    })? {
        return Err(ProbeValidationError {
            message: "A probe with this data column index already exists on this tray".to_string(),
            field: "data_column_index".to_string(),
        });
    }

    Ok(())
}

pub fn validate_probe_data(name: &str, data_column_index: i32) -> Result<(), ProbeValidationError> {
    // Validate name is not empty
    if name.trim().is_empty() {
        return Err(ProbeValidationError {
            message: "Probe name cannot be empty".to_string(),
            field: "name".to_string(),
        });
    }

    // Validate data column index is positive
    if data_column_index < 1 {
        return Err(ProbeValidationError {
            message: "Data column index must be a positive integer".to_string(),
            field: "data_column_index".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_probe_data() {
        // Valid data should pass
        assert!(validate_probe_data("Probe 1", 1).is_ok());
        
        // Empty name should fail
        assert!(validate_probe_data("", 1).is_err());
        assert!(validate_probe_data("   ", 1).is_err());
        
        // Invalid column index should fail
        assert!(validate_probe_data("Probe 1", 0).is_err());
        assert!(validate_probe_data("Probe 1", -1).is_err());
    }
}