use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::DbErr;
use serde_json::json;
use std::fmt;

/// Custom error types for business logic validation and application errors
#[derive(Debug, Clone)]
pub enum BusinessError {
    /// Validation errors for user input (400 Bad Request)
    ValidationError { field: String, message: String },
    /// Business rule violations (422 Unprocessable Entity) 
    BusinessRuleViolation { rule: String, message: String },
    /// Resource not found (404 Not Found)
    NotFound { resource: String, id: String },
    /// Duplicate resource (409 Conflict)
    Duplicate { resource: String, field: String },
    /// Permission denied (403 Forbidden)
    Forbidden { action: String, resource: String },
    /// External service errors (502 Bad Gateway)
    ExternalServiceError { service: String, message: String },
    /// Generic application error (500 Internal Server Error)
    InternalError { message: String },
}

impl fmt::Display for BusinessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BusinessError::ValidationError { field, message } => {
                write!(f, "Validation error in field '{}': {}", field, message)
            }
            BusinessError::BusinessRuleViolation { rule, message } => {
                write!(f, "Business rule '{}' violated: {}", rule, message)
            }
            BusinessError::NotFound { resource, id } => {
                write!(f, "{} with id '{}' not found", resource, id)
            }
            BusinessError::Duplicate { resource, field } => {
                write!(f, "{} with this {} already exists", resource, field)
            }
            BusinessError::Forbidden { action, resource } => {
                write!(f, "Not authorized to {} {}", action, resource)
            }
            BusinessError::ExternalServiceError { service, message } => {
                write!(f, "External service '{}' error: {}", service, message)
            }
            BusinessError::InternalError { message } => {
                write!(f, "Internal error: {}", message)
            }
        }
    }
}

impl std::error::Error for BusinessError {}

/// Convert BusinessError to HTTP responses
impl IntoResponse for BusinessError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            BusinessError::ValidationError { field, message } => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_ERROR",
                format!("Validation failed for field '{}': {}", field, message),
            ),
            BusinessError::BusinessRuleViolation { rule, message } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "BUSINESS_RULE_VIOLATION", 
                format!("Business rule '{}' violated: {}", rule, message),
            ),
            BusinessError::NotFound { resource, id } => (
                StatusCode::NOT_FOUND,
                "RESOURCE_NOT_FOUND",
                format!("{} with id '{}' not found", resource, id),
            ),
            BusinessError::Duplicate { resource, field } => (
                StatusCode::CONFLICT,
                "DUPLICATE_RESOURCE",
                format!("{} with this {} already exists", resource, field),
            ),
            BusinessError::Forbidden { action, resource } => (
                StatusCode::FORBIDDEN,
                "FORBIDDEN",
                format!("Not authorized to {} {}", action, resource),
            ),
            BusinessError::ExternalServiceError { service, message } => (
                StatusCode::BAD_GATEWAY,
                "EXTERNAL_SERVICE_ERROR",
                format!("External service '{}' error: {}", service, message),
            ),
            BusinessError::InternalError { message } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                format!("Internal error: {}", message),
            ),
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message,
                "type": format!("{:?}", self).split('{').next().unwrap_or("Unknown")
            }
        }));

        (status, body).into_response()
    }
}

/// Custom error mapper for crudcrate integration
pub struct ErrorMapper;

impl ErrorMapper {
    /// Map DbErr to appropriate HTTP responses with business context
    pub fn map_db_error(err: DbErr, context: &str) -> BusinessError {
        match err {
            DbErr::RecordNotFound(msg) => {
                // Extract resource type from context or error message
                let resource = Self::extract_resource_from_context(context);
                let id = Self::extract_id_from_message(&msg);
                BusinessError::NotFound { resource, id }
            }
            DbErr::Custom(msg) => {
                // Parse custom error messages for specific business errors
                if msg.starts_with("Validation failed:") {
                    let field = Self::extract_field_from_validation(&msg);
                    let message = msg.replace("Validation failed:", "").trim().to_string();
                    BusinessError::ValidationError { field, message }
                } else if msg.contains("already exists") || msg.contains("duplicate") {
                    let resource = Self::extract_resource_from_context(context);
                    let field = Self::extract_field_from_duplicate(&msg);
                    BusinessError::Duplicate { resource, field }
                } else if msg.contains("Business rule") {
                    let rule = Self::extract_rule_from_message(&msg);
                    BusinessError::BusinessRuleViolation { rule, message: msg }
                } else {
                    BusinessError::InternalError { message: msg }
                }
            }
            DbErr::Conn(conn_err) => {
                BusinessError::ExternalServiceError {
                    service: "database".to_string(),
                    message: conn_err.to_string(),
                }
            }
            DbErr::Exec(exec_err) => {
                // Check if it's a constraint violation
                let err_msg = exec_err.to_string();
                if err_msg.contains("UNIQUE constraint") || err_msg.contains("duplicate key") {
                    let resource = Self::extract_resource_from_context(context);
                    let field = Self::extract_field_from_constraint(&err_msg);
                    BusinessError::Duplicate { resource, field }
                } else {
                    BusinessError::InternalError { message: err_msg }
                }
            }
            _ => BusinessError::InternalError {
                message: err.to_string(),
            },
        }
    }

    /// Helper to extract resource name from context
    fn extract_resource_from_context(context: &str) -> String {
        // Extract resource name from context like "tray_configuration", "experiment", etc.
        context.replace('_', " ").to_string()
    }

    /// Helper to extract ID from error messages
    fn extract_id_from_message(msg: &str) -> String {
        // Look for patterns like "id 'value'" or "ID 'value'" 
        if let Some(start_pos) = msg.find(" id '") {
            let after_id = &msg[start_pos + 5..]; // Skip " id '"
            if let Some(end_pos) = after_id.find('\'') {
                return after_id[..end_pos].to_string();
            }
        }
        
        // Look for patterns with double quotes
        if let Some(start_pos) = msg.find(" id \"") {
            let after_id = &msg[start_pos + 5..]; // Skip " id \""
            if let Some(end_pos) = after_id.find('"') {
                return after_id[..end_pos].to_string();
            }
        }
        
        // Fallback: Try to extract UUID or ID from error message, handling quotes
        msg.split_whitespace()
            .find_map(|word| {
                // Remove surrounding quotes
                let cleaned = word.trim_matches('\'').trim_matches('"');
                if cleaned.len() == 36 && cleaned.matches('-').count() == 4 // UUID format
                    || cleaned.parse::<i32>().is_ok() // Integer ID  
                {
                    Some(cleaned.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Helper to extract field name from validation error
    fn extract_field_from_validation(msg: &str) -> String {
        // Extract field name from messages like "Validation failed: qty_x_axis must be positive"
        msg.split(':')
            .nth(1)
            .and_then(|part| part.trim().split_whitespace().next())
            .unwrap_or("unknown")
            .to_string()
    }

    /// Helper to extract field from duplicate error
    fn extract_field_from_duplicate(msg: &str) -> String {
        if msg.contains("name") {
            "name".to_string()
        } else if msg.contains("email") {
            "email".to_string()
        } else {
            "field".to_string()
        }
    }

    /// Helper to extract rule from business rule violation
    fn extract_rule_from_message(msg: &str) -> String {
        msg.split("Business rule")
            .nth(1)
            .and_then(|part| part.split("violated").next())
            .unwrap_or("unknown")
            .trim()
            .trim_matches('\'')
            .trim_matches('"')
            .to_string()
    }

    /// Helper to extract field from constraint violation
    fn extract_field_from_constraint(msg: &str) -> String {
        // Extract field name from constraint error messages
        if msg.contains("name") {
            "name".to_string()
        } else if msg.contains("email") {
            "email".to_string()
        } else {
            "field".to_string()
        }
    }
}

/// Convenience macros for creating business errors
#[macro_export]
macro_rules! validation_error {
    ($field:expr, $message:expr) => {
        crate::common::errors::BusinessError::ValidationError {
            field: $field.to_string(),
            message: $message.to_string(),
        }
    };
}

#[macro_export]
macro_rules! business_rule_violation {
    ($rule:expr, $message:expr) => {
        crate::common::errors::BusinessError::BusinessRuleViolation {
            rule: $rule.to_string(),
            message: $message.to_string(),
        }
    };
}

#[macro_export]
macro_rules! not_found {
    ($resource:expr, $id:expr) => {
        crate::common::errors::BusinessError::NotFound {
            resource: $resource.to_string(),
            id: $id.to_string(),
        }
    };
}

#[macro_export]
macro_rules! duplicate_resource {
    ($resource:expr, $field:expr) => {
        crate::common::errors::BusinessError::Duplicate {
            resource: $resource.to_string(),
            field: $field.to_string(),
        }
    };
}

/// Extension trait to add business error conversion to DbErr
pub trait DbErrorExt {
    fn to_business_error(self, context: &str) -> BusinessError;
}

impl DbErrorExt for DbErr {
    fn to_business_error(self, context: &str) -> BusinessError {
        ErrorMapper::map_db_error(self, context)
    }
}

/// Result type alias for business operations
pub type BusinessResult<T> = Result<T, BusinessError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_creation() {
        let err = validation_error!("qty_x_axis", "must be positive");
        assert!(matches!(err, BusinessError::ValidationError { .. }));
    }

    #[test]
    fn test_error_mapper_validation() {
        let db_err = DbErr::Custom("Validation failed: qty_x_axis must be positive".to_string());
        let business_err = ErrorMapper::map_db_error(db_err, "tray_configuration");
        
        match business_err {
            BusinessError::ValidationError { field, message } => {
                assert_eq!(field, "qty_x_axis");
                assert!(message.contains("must be positive"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_error_mapper_not_found() {
        let db_err = DbErr::RecordNotFound("Tray configuration with id 'abc-123' not found".to_string());
        let business_err = ErrorMapper::map_db_error(db_err, "tray_configuration");
        
        match business_err {
            BusinessError::NotFound { resource, id } => {
                assert_eq!(resource, "tray configuration");
                assert_eq!(id, "abc-123");
            }
            _ => panic!("Expected not found error"),
        }
    }
}