use application::roles::dto::RoleDto;
use application::users::dto::{AuthenticateUserResult, ListUsersResult, UserDto};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(email(message = "invalid email"))]
    pub email: String,
    #[validate(length(min = 3, max = 32, message = "username must be 3-32 characters"))]
    pub username: String,
    #[validate(length(min = 8, message = "password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct CreateUserResponse {
    pub id: Uuid,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(email(message = "invalid email"))]
    pub email: Option<String>,
    #[validate(length(min = 3, max = 32, message = "username must be 3-32 characters"))]
    pub username: Option<String>,
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AssignUserRolesRequest {
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateRoleScopesRequest {
    pub scopes: Vec<String>,
}

#[derive(Debug, Deserialize, Validate, Default)]
pub struct ListUsersQuery {
    #[validate(range(min = 1, message = "page must be at least 1"))]
    pub page: Option<i64>,
    #[validate(range(min = 1, max = 100, message = "per_page must be between 1 and 100"))]
    pub per_page: Option<i64>,
    pub active_only: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "invalid email"))]
    pub email: String,
    #[validate(length(min = 8, message = "password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user_id: Uuid,
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse<'a> {
    pub user: &'a UserDto,
}

impl<'a> From<&'a UserDto> for UserResponse<'a> {
    fn from(user: &'a UserDto) -> Self {
        Self { user }
    }
}

#[derive(Debug, Serialize)]
pub struct UsersResponse {
    pub users: Vec<UserDto>,
    pub total: usize,
    pub page: i64,
    pub per_page: i64,
}

impl From<ListUsersResult> for UsersResponse {
    fn from(result: ListUsersResult) -> Self {
        Self {
            users: result.users,
            total: result.total,
            page: result.page,
            per_page: result.per_page,
        }
    }
}

impl From<AuthenticateUserResult> for LoginResponse {
    fn from(result: AuthenticateUserResult) -> Self {
        Self {
            user_id: result.user_id,
            token: result.token,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RolesResponse {
    pub roles: Vec<RoleDto>,
}

impl From<Vec<RoleDto>> for RolesResponse {
    fn from(roles: Vec<RoleDto>) -> Self {
        Self { roles }
    }
}

#[derive(Debug, Serialize)]
pub struct VerifyEmailResponse {
    pub status: String,
    pub user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ResetPasswordResponse {
    pub status: String,
    pub user_id: Uuid,
}
