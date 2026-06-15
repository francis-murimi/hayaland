use actix_web::{HttpMessage, HttpRequest};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;

pub mod admin_broadcast;
pub mod delete_message;
pub mod edit_message;
pub mod get_message;
pub mod list_conversations;
pub mod list_messages;
pub mod mark_read;
pub mod pin_message;
pub mod react;
pub mod remove_reaction;
pub mod send_message;
pub mod unpin_message;
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

pub(crate) fn is_message_admin(ctx: &AuthContext) -> bool {
    ctx.has_scope("admin:messages") || ctx.has_scope("admin:*")
}

pub(crate) fn require_message_admin(ctx: &AuthContext) -> Result<(), ApiError> {
    if is_message_admin(ctx) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}
