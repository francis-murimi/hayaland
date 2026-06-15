use actix_web::{HttpMessage, HttpRequest};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;

pub mod create_chat_room;
pub mod delete_chat_room;
pub mod get_chat_room;
pub mod join_chat_room;
pub mod leave_chat_room;
pub mod list_chat_rooms;
pub mod list_room_messages;
pub mod manage_membership;
pub mod update_chat_room;

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

pub(crate) fn is_chatroom_admin(ctx: &AuthContext) -> bool {
    ctx.has_scope("admin:messages") || ctx.has_scope("admin:*")
}

pub(crate) fn actor_party_ids(req: &HttpRequest) -> Vec<Uuid> {
    actor_party_id(req).into_iter().collect()
}
