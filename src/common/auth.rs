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
