use crate::errors::ApplicationError;
use crate::messages::access::is_message_visible_to_actor;
use crate::messages::dto::{to_message_result, MessageResult};
use crate::ports::EncryptionService;
use domain::repositories::{
    ChatRoomRepository, DealRepository, MessageRepository, PartyRepository,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct GetMessage {
    message_repo: Arc<dyn MessageRepository>,
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    room_repo: Arc<dyn ChatRoomRepository>,
    encryption: Arc<dyn EncryptionService>,
}

impl GetMessage {
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
        message_id: Uuid,
        actor_user_id: Uuid,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<MessageResult, ApplicationError> {
        let message = self
            .message_repo
            .find_message_by_id(message_id)
            .await?
            .ok_or(ApplicationError::MessageNotFound)?;

        if !is_message_visible_to_actor(
            &message,
            actor_user_id,
            actor_party_id,
            is_admin,
            self.party_repo.as_ref(),
            self.deal_repo.as_ref(),
            self.room_repo.as_ref(),
        )
        .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        to_message_result(message, &self.encryption).await
    }
}
