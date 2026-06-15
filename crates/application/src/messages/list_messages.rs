use crate::errors::ApplicationError;
use crate::messages::access::is_conversation_visible_to_actor;
use crate::messages::dto::{to_message_result, ListMessagesQuery, MessageResult};
use crate::ports::EncryptionService;
use domain::repositories::{
    ChatRoomRepository, DealRepository, MessageRepository, PartyRepository,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ListMessages {
    message_repo: Arc<dyn MessageRepository>,
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    room_repo: Arc<dyn ChatRoomRepository>,
    encryption: Arc<dyn EncryptionService>,
}

impl ListMessages {
    pub fn new(
        message_repo: Arc<dyn MessageRepository>,
        party_repo: Arc<dyn PartyRepository>,
        deal_repo: Arc<dyn DealRepository>,
        room_repo: Arc<dyn ChatRoomRepository>,
        encryption: Arc<dyn EncryptionService>,
    ) -> Self {
        Self {
            message_repo,
            party_repo,
            deal_repo,
            room_repo,
            encryption,
        }
    }

    pub async fn execute(
        &self,
        conversation_id: Uuid,
        query: ListMessagesQuery,
    ) -> Result<Vec<MessageResult>, ApplicationError> {
        let conversation = self
            .message_repo
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or(ApplicationError::ConversationNotFound)?;

        if !is_conversation_visible_to_actor(
            &conversation,
            query.actor_user_id,
            query.actor_party_id,
            query.is_admin,
            self.party_repo.as_ref(),
            self.deal_repo.as_ref(),
            self.room_repo.as_ref(),
        )
        .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        let domain_query = domain::repositories::MessageListQuery {
            before_id: query.before_id,
            limit: query.limit,
        };
        let with_meta = self
            .message_repo
            .list_messages(conversation_id, &domain_query)
            .await?;

        let mut results = Vec::new();
        for item in with_meta {
            results.push(to_message_result(item.message, &self.encryption).await?);
        }
        Ok(results)
    }
}
