use crate::errors::ApplicationError;
use crate::messages::dto::{ConversationResult, ListConversationsQuery};
use domain::repositories::MessageRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct ListConversations {
    message_repo: Arc<dyn MessageRepository>,
}

impl ListConversations {
    pub fn new(message_repo: Arc<dyn MessageRepository>) -> Self {
        Self { message_repo }
    }

    pub async fn execute(
        &self,
        query: ListConversationsQuery,
    ) -> Result<Vec<ConversationResult>, ApplicationError> {
        let summaries = self
            .message_repo
            .list_conversations_for_user(
                query.actor_user_id,
                query.actor_party_id,
                query.limit,
                query.offset,
            )
            .await?;

        Ok(summaries
            .into_iter()
            .map(|s| ConversationResult {
                id: s.conversation.id,
                conversation_type: s.conversation.conversation_type,
                user_a_id: s.conversation.user_a_id,
                user_b_id: s.conversation.user_b_id,
                party_a_id: s.conversation.party_a_id,
                party_b_id: s.conversation.party_b_id,
                party_id: s.conversation.party_id,
                deal_id: s.conversation.deal_id,
                room_id: s.conversation.room_id,
                title: s.conversation.title,
                last_message_at: s.conversation.last_message_at,
                unread_count: s.unread_count,
            })
            .collect())
    }
}
