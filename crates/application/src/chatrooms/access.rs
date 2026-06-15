use crate::errors::ApplicationError;
use domain::entities::{ChatRoom, ChatRoomType};
use domain::repositories::ChatRoomRepository;
use uuid::Uuid;

pub fn has_scope(scopes: &[String], scope: &str) -> bool {
    scopes.iter().any(|s| s == scope || s == "admin:*")
}

pub fn can_create_chat_room(scopes: &[String]) -> bool {
    has_scope(scopes, "chatrooms:write")
        || has_scope(scopes, "chatrooms:moderate")
        || has_scope(scopes, "admin:messages")
}

pub async fn can_manage_chat_room(
    room: &ChatRoom,
    actor_user_id: Uuid,
    is_admin: bool,
    room_repo: &dyn ChatRoomRepository,
) -> Result<bool, ApplicationError> {
    if is_admin || room.created_by_user_id == actor_user_id {
        return Ok(true);
    }
    if let Some(membership) = room_repo
        .find_membership_for_user(room.id, actor_user_id)
        .await?
    {
        return Ok(membership.is_moderator());
    }
    Ok(false)
}

pub async fn is_chat_room_visible(
    room: &ChatRoom,
    actor_user_id: Uuid,
    actor_party_ids: &[Uuid],
    is_admin: bool,
    room_repo: &dyn ChatRoomRepository,
) -> Result<bool, ApplicationError> {
    if is_admin {
        return Ok(true);
    }
    if room.is_deleted {
        return Ok(false);
    }
    if room.room_type == ChatRoomType::Public {
        return Ok(true);
    }
    if room_repo
        .is_user_in_room(room.id, actor_user_id, actor_party_ids)
        .await?
    {
        return Ok(true);
    }
    Ok(false)
}
