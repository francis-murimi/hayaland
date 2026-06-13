use application::parties::dto::{PartyResult, PartySummaryResult, RoleResult, SearchPartiesResult};
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

// ============================================================================
// Party DTOs
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePartyRequest {
    #[validate(length(min = 3, max = 120, message = "display name must be 3-120 characters"))]
    pub display_name: String,
    #[validate(email(message = "invalid email"))]
    pub email: String,
    pub party_type: String,
    pub phone: Option<String>,
    pub tax_id: Option<String>,
    pub primary_domain_id: Option<Uuid>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<f64>,
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePartyRequest {
    #[validate(length(min = 3, max = 120, message = "display name must be 3-120 characters"))]
    pub display_name: Option<String>,
    #[validate(email(message = "invalid email"))]
    pub email: Option<String>,
    pub phone: Option<String>,
    pub tax_id: Option<String>,
    pub primary_domain_id: Option<Uuid>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<f64>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AddPartyRoleRequest {
    pub role_type: String,
    pub profile: serde_json::Value,
}

#[derive(Debug, Deserialize, Validate, Default)]
pub struct SearchPartiesQuery {
    pub q: Option<String>,
    pub roles: Option<Vec<String>>,
    pub party_types: Option<Vec<String>>,
    pub verification_statuses: Option<Vec<String>>,
    pub min_trust_score: Option<f64>,
    pub max_trust_score: Option<f64>,
    pub primary_domain_id: Option<Uuid>,
    pub active_only: Option<bool>,
    #[serde(rename = "lat")]
    pub latitude: Option<f64>,
    #[serde(rename = "lng")]
    pub longitude: Option<f64>,
    #[serde(rename = "radiusKm")]
    pub radius_km: Option<f64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PartyResponse {
    #[serde(flatten)]
    pub party: PartyResult,
}

impl From<PartyResult> for PartyResponse {
    fn from(party: PartyResult) -> Self {
        Self { party }
    }
}

#[derive(Debug, Serialize)]
pub struct PartiesResponse {
    pub parties: Vec<PartySummaryResult>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

impl From<SearchPartiesResult> for PartiesResponse {
    fn from(result: SearchPartiesResult) -> Self {
        Self {
            parties: result.parties,
            total: result.total,
            limit: result.limit,
            offset: result.offset,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PartyRolesResponse {
    pub roles: Vec<RoleResult>,
}

impl From<Vec<RoleResult>> for PartyRolesResponse {
    fn from(roles: Vec<RoleResult>) -> Self {
        Self { roles }
    }
}

#[derive(Debug, Serialize)]
pub struct MyPartiesResponse {
    pub parties: Vec<PartySummaryResult>,
}
