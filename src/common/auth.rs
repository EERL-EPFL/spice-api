#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Role {
    Administrator,
    Unknown(String),
}
impl axum_keycloak_auth::role::Role for Role {}
impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let config = crate::config::Config::from_env();
        match self {
            Role::Administrator => f.write_str(config.admin_role.as_str()),
            Role::Unknown(unknown) => f.write_fmt(format_args!("Unknown role: {unknown}")),
        }
    }
}

impl From<String> for Role {
    fn from(value: String) -> Self {
        let config = crate::config::Config::from_env();
        let admin_role = config.admin_role.as_str();
        if value == admin_role {
            Role::Administrator
        } else {
            Role::Unknown(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_role_variants_exist() {
        // Test that Role enum variants can be created and have expected properties
        let admin = Role::Administrator;
        let unknown = Role::Unknown("test".to_string());
        
        // Test Debug trait
        assert_eq!(format!("{:?}", admin), "Administrator");
        assert!(format!("{:?}", unknown).contains("Unknown"));
        
        // Test Clone and PartialEq
        let admin2 = admin.clone();
        assert_eq!(admin, admin2);
        assert_ne!(admin, unknown);
    }

    #[test]
    fn test_role_unknown_creation() {
        // Test creating Unknown role variants
        let role1 = Role::Unknown("test_role".to_string());
        let role2 = Role::Unknown("different_role".to_string());
        let role3 = Role::Unknown("test_role".to_string());
        
        // Test equality
        assert_eq!(role1, role3);
        assert_ne!(role1, role2);
        
        // Test clone
        let role1_clone = role1.clone();
        assert_eq!(role1, role1_clone);
    }

    #[test]
    fn test_role_trait_implementations() {
        // Test that Role implements required traits without environment dependencies
        let admin = Role::Administrator;
        let unknown = Role::Unknown("test".to_string());
        
        // Test Debug trait
        let admin_debug = format!("{:?}", admin);
        assert!(admin_debug.contains("Administrator"));
        
        let unknown_debug = format!("{:?}", unknown);
        assert!(unknown_debug.contains("Unknown"));
        assert!(unknown_debug.contains("test"));
    }

    #[test]
    fn test_role_enum_pattern_matching() {
        // Test pattern matching on Role enum
        let admin = Role::Administrator;
        let unknown = Role::Unknown("test".to_string());
        
        match admin {
            Role::Administrator => assert!(true),
            Role::Unknown(_) => panic!("Expected Administrator"),
        }
        
        match unknown {
            Role::Administrator => panic!("Expected Unknown"),
            Role::Unknown(value) => assert_eq!(value, "test"),
        }
    }

    #[test]
    fn test_role_implements_required_traits() {
        // Test that Role implements axum_keycloak_auth::role::Role trait (compilation test)
        // This ensures our Role enum can be used with the keycloak auth system
        let role = Role::Administrator;
        
        // These calls ensure the traits are implemented and compiles successfully
        let _debug = format!("{:?}", role);
        let _clone = role.clone();
        
        // Test that we can create different role variants
        let _unknown = Role::Unknown("test_role".to_string());
    }
}
