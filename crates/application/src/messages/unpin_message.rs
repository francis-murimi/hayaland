use crate::errors::ApplicationError;
use crate::messages::access::is_message_visible_to_actor;
use crate::messages::dto::{to_message_result, MessageResult, PinMessageCommand};
use crate::ports::EncryptionService;
use domain::repositories::{
    ChatRoomRepository, DealRepository, MessageRepository, PartyRepository,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct UnpinMessage {
    message_repo: Arc<dyn MessageRepository>,
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    room_repo: Arc<dyn ChatRoomRepository>,
    encryption: Arc<dyn EncryptionService>,
}

impl UnpinMessage {
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

    pub async fn execute(&self, cmd: PinMessageCommand) -> Result<MessageResult, ApplicationError> {
        let mut message = self
            .message_repo
            .find_message_by_id(cmd.message_id)
            .await?
            .ok_or(ApplicationError::MessageNotFound)?;

        if !is_message_visible_to_actor(
            &message,
            cmd.actor_user_id,
            cmd.actor_party_id,
            cmd.is_admin,
            self.party_repo.as_ref(),
            self.deal_repo.as_ref(),
            self.room_repo.as_ref(),
        )
        .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        if !message.can_be_edited_by(cmd.actor_user_id, cmd.is_admin) {
            return Err(ApplicationError::Forbidden);
        }

        message.unpin();
        self.message_repo
            .set_message_pinned(message.id, false, None)
            .await?;

        to_message_result(message, &self.encryption).await
    }
}
