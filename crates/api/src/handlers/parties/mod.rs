use crate::errors::ApiError;
use application::users::token::AuthContext;
use domain::entities::DealRole;

pub mod add_role;
pub mod create_party;
pub mod delete_party;
pub mod get_party;
pub mod list_my_parties;
pub mod list_parties;
pub mod list_roles;
pub mod nearby;
pub mod remove_role;
pub mod search_parties;
pub mod update_party;

/// Check whether the authenticated user is a platform admin.
pub(crate) fn is_admin(ctx: &AuthContext) -> bool {
    ctx.has_scope("admin:parties") || ctx.has_scope("admin:*")
}

/// Parse a deal role string into the domain enum.
pub(crate) fn parse_role(role: &str) -> Result<DealRole, ApiError> {
    DealRole::try_from(role).map_err(|_| {
        ApiError::Validation(format!(
            "invalid role '{role}', expected SUPPLIER, CONSUMER, or ENHANCER"
        ))
    })
}
