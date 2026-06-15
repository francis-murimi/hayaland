use crate::entities::{Conversation, Message, MessageReaction, MessageRead, RecipientType};
use crate::errors::DomainError;
use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

/// Pagination / filtering for listing messages.
#[derive(Debug, Clone, Default)]
pub struct MessageListQuery {
    pub before_id: Option<Uuid>,
    pub limit: i64,
}

/// A message enriched with read/reaction metadata.
#[derive(Debug, Clone)]
pub struct MessageWithMeta {
    pub message: Message,
    pub read_count: i64,
    pub likes: i64,
    pub dislikes: i64,
    pub user_reaction: Option<MessageReaction>,
}

/// A lightweight conversation summary for a participant.
#[derive(Debug, Clone)]
pub struct ConversationSummary {
    pub conversation: Conversation,
    pub unread_count: i64,
}

/// Outbound port for persisting and retrieving messages, conversations, reads and reactions.
#[async_trait]
pub trait MessageRepository: Send + Sync {
    /// Create a conversation.
    async fn create_conversation(&self, conversation: &Conversation) -> Result<(), DomainError>;

    /// Find a conversation by its id.
    async fn find_conversation_by_id(&self, id: Uuid) -> Result<Option<Conversation>, DomainError>;

    /// Find a direct-user conversation between two users.
    async fn find_direct_user_conversation(
        &self,
        user_a_id: Uuid,
        user_b_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError>;

    /// Find a direct-party conversation between two parties.
    async fn find_direct_party_conversation(
        &self,
        party_a_id: Uuid,
        party_b_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError>;

    /// Find a party-members conversation for a party.
    async fn find_party_members_conversation(
        &self,
        party_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError>;

    /// Find a deal conversation.
    async fn find_deal_conversation(
        &self,
        deal_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError>;

    /// Find a room conversation.
    async fn find_room_conversation(
        &self,
        room_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError>;

    /// Update `last_message_at` for a conversation.
    async fn touch_conversation(
        &self,
        conversation_id: Uuid,
        last_message_at: OffsetDateTime,
    ) -> Result<(), DomainError>;

    /// Create a message.
    async fn create_message(&self, message: &Message) -> Result<(), DomainError>;

    /// Find a message by id.
    async fn find_message_by_id(&self, id: Uuid) -> Result<Option<Message>, DomainError>;

    /// List messages in a conversation that the caller is authorized to see.
    async fn list_messages(
        &self,
        conversation_id: Uuid,
        query: &MessageListQuery,
    ) -> Result<Vec<MessageWithMeta>, DomainError>;

    /// Update message content and `edited_at`.
    async fn update_message(&self, message: &Message) -> Result<(), DomainError>;

    /// Soft-delete a message.
    async fn soft_delete_message(&self, id: Uuid) -> Result<(), DomainError>;

    /// Pin or unpin a message.
    async fn set_message_pinned(
        &self,
        message_id: Uuid,
        is_pinned: bool,
        pinned_at: Option<OffsetDateTime>,
    ) -> Result<(), DomainError>;

    /// List pinned messages in a conversation.
    async fn list_pinned_messages(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<MessageWithMeta>, DomainError>;

    /// Mark a message as read for a user.
    async fn mark_read(&self, read: &MessageRead) -> Result<(), DomainError>;

    /// Find an existing read receipt.
    async fn find_read(
        &self,
        message_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<MessageRead>, DomainError>;

    /// Count unread messages for a participant across all visible conversations.
    async fn unread_count_for_user(
        &self,
        user_id: Uuid,
        party_id: Option<Uuid>,
    ) -> Result<i64, DomainError>;

    /// List conversations visible to a user, ordered by `last_message_at DESC`.
    async fn list_conversations_for_user(
        &self,
        user_id: Uuid,
        party_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ConversationSummary>, DomainError>;

    /// Add or remove a reaction. Returns the current reaction if it was added.
    async fn toggle_reaction(
        &self,
        reaction: &MessageReaction,
    ) -> Result<Option<MessageReaction>, DomainError>;

    /// List reactions for a message.
    async fn list_reactions_for_message(
        &self,
        message_id: Uuid,
    ) -> Result<Vec<MessageReaction>, DomainError>;

    /// Count messages of a given recipient type for a target.
    async fn count_messages_by_recipient(
        &self,
        recipient_type: RecipientType,
        recipient_id: Uuid,
    ) -> Result<i64, DomainError>;
}
