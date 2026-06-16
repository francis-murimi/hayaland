use actix_web::{HttpMessage, HttpRequest};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;

pub mod admin_send;
pub mod admin_templates;
pub mod delete_notification;
pub mod get_notification;
pub mod list_notifications;
pub mod mark_all_read;
pub mod mark_read;
pub mod preferences;
pub mod unread_count;

pub(crate) fn extract_ctx(req: &HttpRequest) -> Result<AuthContext, ApiError> {
    req.extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))
}

pub(crate) fn actor_party_id(req: &HttpRequest) -> Option<Uuid> {
    req.headers()
        .get("X-Party-ID")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
}

pub(crate) fn require_notification_admin(ctx: &AuthContext) -> Result<(), ApiError> {
    if ctx.has_scope("admin:notifications") || ctx.has_scope("admin:*") {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}
