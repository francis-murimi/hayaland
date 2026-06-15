use domain::entities::{ChatRoomMemberRole, ChatRoomType};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Command to create a chatroom.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateChatRoomCommand {
    pub actor_user_id: Uuid,
    pub scopes: Vec<String>,
    pub name: String,
    pub description: Option<String>,
    pub room_type: ChatRoomType,
}

/// Command to update a chatroom.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChatRoomCommand {
    pub actor_user_id: Uuid,
    pub scopes: Vec<String>,
    pub is_admin: bool,
    pub room_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub room_type: Option<ChatRoomType>,
}

/// Command to soft-delete a chatroom.
#[derive(Debug, Clone, Deserialize)]
pub struct SoftDeleteChatRoomCommand {
    pub actor_user_id: Uuid,
    pub scopes: Vec<String>,
    pub is_admin: bool,
    pub room_id: Uuid,
}

/// Command to join a chatroom.
#[derive(Debug, Clone, Deserialize)]
pub struct JoinChatRoomCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub scopes: Vec<String>,
    pub is_admin: bool,
    pub room_id: Uuid,
}

/// Command to leave a chatroom.
#[derive(Debug, Clone, Deserialize)]
pub struct LeaveChatRoomCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub room_id: Uuid,
}

/// Action for managing a chatroom membership.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MembershipAction {
    Add,
    Remove,
    SetRole,
}

/// Command to manage a chatroom membership.
#[derive(Debug, Clone, Deserialize)]
pub struct ManageMembershipCommand {
    pub actor_user_id: Uuid,
    pub scopes: Vec<String>,
    pub is_admin: bool,
    pub room_id: Uuid,
    pub action: MembershipAction,
    pub target_user_id: Option<Uuid>,
    pub target_party_id: Option<Uuid>,
    pub role: Option<ChatRoomMemberRole>,
    pub membership_id: Option<Uuid>,
}

/// A chatroom as returned by application use cases.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRoomResult {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub room_type: ChatRoomType,
    pub created_by_user_id: Uuid,
    pub is_deleted: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<domain::entities::ChatRoom> for ChatRoomResult {
    fn from(r: domain::entities::ChatRoom) -> Self {
        Self {
            id: r.id,
            name: r.name.as_str().to_string(),
            description: r.description,
            room_type: r.room_type,
            created_by_user_id: r.created_by_user_id,
            is_deleted: r.is_deleted,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// A chatroom membership as returned by application use cases.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRoomMembershipResult {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: Option<Uuid>,
    pub party_id: Option<Uuid>,
    pub member_role: ChatRoomMemberRole,
    pub joined_at: OffsetDateTime,
}

impl From<domain::entities::ChatRoomMembership> for ChatRoomMembershipResult {
    fn from(m: domain::entities::ChatRoomMembership) -> Self {
        Self {
            id: m.id,
            room_id: m.room_id,
            user_id: m.user_id,
            party_id: m.party_id,
            member_role: m.member_role,
            joined_at: m.joined_at,
        }
    }
}

/// Query parameters for listing chatrooms.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChatRoomListQuery {
    pub room_type: Option<ChatRoomType>,
    pub include_deleted: bool,
    pub limit: i64,
    pub offset: i64,
}
