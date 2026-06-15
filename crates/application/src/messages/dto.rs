use crate::errors::ApplicationError;
use crate::ports::EncryptionService;
use domain::entities::{
    ChatRoomMemberRole, ConversationType, Message, MessageType, ReactionType, RecipientType,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Command to send a message in any supported context.
#[derive(Debug, Clone, Deserialize)]
pub struct SendMessageCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub scopes: Vec<String>,
    pub is_admin: bool,
    pub recipient_type: RecipientType,
    pub recipient_user_id: Option<Uuid>,
    pub recipient_party_id: Option<Uuid>,
    pub recipient_deal_id: Option<Uuid>,
    pub recipient_room_id: Option<Uuid>,
    pub message_type: MessageType,
    pub subject: Option<String>,
    pub content: String,
    pub attachment_urls: Vec<String>,
    pub reply_to_message_id: Option<Uuid>,
}

/// Command to edit an existing message.
#[derive(Debug, Clone, Deserialize)]
pub struct EditMessageCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub scopes: Vec<String>,
    pub is_admin: bool,
    pub message_id: Uuid,
    pub content: String,
}

/// Command to soft-delete a message.
#[derive(Debug, Clone, Deserialize)]
pub struct SoftDeleteMessageCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub scopes: Vec<String>,
    pub is_admin: bool,
    pub message_id: Uuid,
}

/// Command to mark a message as read.
#[derive(Debug, Clone, Deserialize)]
pub struct MarkReadCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub scopes: Vec<String>,
    pub is_admin: bool,
    pub message_id: Uuid,
}

/// Command to toggle a reaction on a message.
#[derive(Debug, Clone, Deserialize)]
pub struct ToggleReactionCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub scopes: Vec<String>,
    pub is_admin: bool,
    pub message_id: Uuid,
    pub reaction_type: ReactionType,
}

/// Command to pin a message.
#[derive(Debug, Clone, Deserialize)]
pub struct PinMessageCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub scopes: Vec<String>,
    pub is_admin: bool,
    pub message_id: Uuid,
}

/// Target audience for an admin broadcast.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BroadcastTarget {
    AllUsers,
    AllParties,
    AllUsersAndParties,
}

/// Command to send an admin broadcast.
#[derive(Debug, Clone, Deserialize)]
pub struct AdminBroadcastCommand {
    pub actor_user_id: Uuid,
    pub scopes: Vec<String>,
    pub target: BroadcastTarget,
    pub subject: Option<String>,
    pub content: String,
}

/// Query parameters for listing messages in a conversation.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListMessagesQuery {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub scopes: Vec<String>,
    pub is_admin: bool,
    pub before_id: Option<Uuid>,
    pub limit: i64,
}

/// Query parameters for listing conversations.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListConversationsQuery {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

/// A message as returned by application use cases.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageResult {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_user_id: Uuid,
    pub sender_party_id: Option<Uuid>,
    pub recipient_type: RecipientType,
    pub recipient_user_id: Option<Uuid>,
    pub recipient_party_id: Option<Uuid>,
    pub recipient_deal_id: Option<Uuid>,
    pub recipient_room_id: Option<Uuid>,
    pub message_type: MessageType,
    pub subject: Option<String>,
    pub content_plaintext: String,
    pub attachment_urls: Vec<String>,
    pub reply_to_message_id: Option<Uuid>,
    pub is_pinned: bool,
    pub pinned_at: Option<OffsetDateTime>,
    pub is_deleted: bool,
    pub edited_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

/// Convert a stored message into a result DTO, decrypting the content.
pub async fn to_message_result(
    message: Message,
    encryption: &Arc<dyn EncryptionService>,
) -> Result<MessageResult, ApplicationError> {
    let content_plaintext = if message.is_deleted {
        String::new()
    } else {
        encryption.decrypt(&message.content).await?
    };
    Ok(MessageResult {
        id: message.id,
        conversation_id: message.conversation_id,
        sender_user_id: message.sender_user_id,
        sender_party_id: message.sender_party_id,
        recipient_type: message.recipient_type,
        recipient_user_id: message.recipient_user_id,
        recipient_party_id: message.recipient_party_id,
        recipient_deal_id: message.recipient_deal_id,
        recipient_room_id: message.recipient_room_id,
        message_type: message.message_type,
        subject: message.subject,
        content_plaintext,
        attachment_urls: message.attachment_urls,
        reply_to_message_id: message.reply_to_message_id,
        is_pinned: message.is_pinned,
        pinned_at: message.pinned_at,
        is_deleted: message.is_deleted,
        edited_at: message.edited_at,
        created_at: message.created_at,
    })
}

/// A conversation as returned by application use cases.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationResult {
    pub id: Uuid,
    pub conversation_type: ConversationType,
    pub user_a_id: Option<Uuid>,
    pub user_b_id: Option<Uuid>,
    pub party_a_id: Option<Uuid>,
    pub party_b_id: Option<Uuid>,
    pub party_id: Option<Uuid>,
    pub deal_id: Option<Uuid>,
    pub room_id: Option<Uuid>,
    pub title: Option<String>,
    pub last_message_at: OffsetDateTime,
    pub unread_count: i64,
}

/// A reaction as returned by application use cases.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageReactionResult {
    pub id: Uuid,
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub party_id: Option<Uuid>,
    pub reaction_type: ReactionType,
    pub created_at: OffsetDateTime,
}

impl From<domain::entities::MessageReaction> for MessageReactionResult {
    fn from(r: domain::entities::MessageReaction) -> Self {
        Self {
            id: r.id,
            message_id: r.message_id,
            user_id: r.user_id,
            party_id: r.party_id,
            reaction_type: r.reaction_type,
            created_at: r.created_at,
        }
    }
}

/// Result of a chat-room membership operation.
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
