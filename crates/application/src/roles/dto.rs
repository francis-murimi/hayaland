use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct RoleDto {
    pub name: String,
    pub scopes: Vec<String>,
    pub is_builtin: bool,
}

impl From<domain::entities::Role> for RoleDto {
    fn from(role: domain::entities::Role) -> Self {
        Self {
            name: role.name,
            scopes: role.scopes,
            is_builtin: role.is_builtin,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRoleScopesCommand {
    pub name: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssignUserRolesCommand {
    pub user_id: uuid::Uuid,
    pub roles: Vec<String>,
}
