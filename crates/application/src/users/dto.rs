use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Input to the create-user use case.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserCommand {
    pub email: String,
    pub username: String,
    pub password: String,
}

/// Output of the create-user use case.
#[derive(Debug, Clone, Serialize)]
pub struct CreateUserResult {
    pub id: Uuid,
}

/// Output of the get-user use case.
#[derive(Debug, Clone, Serialize)]
pub struct UserDto {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub is_active: bool,
    pub roles: Vec<String>,
    pub protected: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<domain::entities::User> for UserDto {
    fn from(user: domain::entities::User) -> Self {
        let format = time::format_description::well_known::Rfc3339;
        Self {
            id: user.id,
            email: user.email.as_str().to_string(),
            username: user.username.as_str().to_string(),
            is_active: user.is_active,
            roles: user.roles,
            protected: user.protected,
            created_at: user
                .created_at
                .format(&format)
                .unwrap_or_else(|_| user.created_at.to_string()),
            updated_at: user
                .updated_at
                .format(&format)
                .unwrap_or_else(|_| user.updated_at.to_string()),
        }
    }
}

/// Input to the list-users use case.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ListUsersQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub active_only: Option<bool>,
}

/// Output of the list-users use case.
#[derive(Debug, Clone, Serialize)]
pub struct ListUsersResult {
    pub users: Vec<UserDto>,
    pub total: usize,
    pub page: i64,
    pub per_page: i64,
}

/// Input to the update-user use case.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateUserCommand {
    pub id: Uuid,
    pub email: Option<String>,
    pub username: Option<String>,
    pub roles: Option<Vec<String>>,
}

/// Input to the deactivate-user use case.
#[derive(Debug, Clone, Deserialize)]
pub struct DeactivateUserCommand {
    pub id: Uuid,
}

/// Input to the authenticate-user (login) use case.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthenticateUserCommand {
    pub email: String,
    pub password: String,
}

/// Output of the authenticate-user use case.
#[derive(Debug, Clone, Serialize)]
pub struct AuthenticateUserResult {
    pub user_id: Uuid,
    pub token: String,
}
