use crate::errors::ApplicationError;
use crate::messages::access::is_message_visible_to_actor;
use crate::messages::dto::MarkReadCommand;
use crate::ports::RealtimePublisher;
use domain::entities::MessageRead;
use domain::repositories::{
    ChatRoomRepository, DealRepository, MessageRepository, PartyRepository,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct MarkRead {
    message_repo: Arc<dyn MessageRepository>,
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    room_repo: Arc<dyn ChatRoomRepository>,
    publisher: Arc<dyn RealtimePublisher>,
}

impl MarkRead {
    pub fn new(
        message_repo: Arc<dyn MessageRepository>,
        party_repo: Arc<dyn PartyRepository>,
        deal_repo: Arc<dyn DealRepository>,
        room_repo: Arc<dyn ChatRoomRepository>,
        publisher: Arc<dyn RealtimePublisher>,
    ) -> Self {
        Self {
            message_repo,
            party_repo,
            deal_repo,
            room_repo,
            publisher,
        }
    }

    pub async fn execute(
        &self,
        cmd: MarkReadCommand,
    ) -> Result<domain::entities::MessageRead, ApplicationError> {
        let message = self
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

        if let Some(existing) = self
            .message_repo
            .find_read(cmd.message_id, cmd.actor_user_id)
            .await?
        {
            return Ok(existing);
        }

        let read = MessageRead::new(
            uuid::Uuid::now_v7(),
            cmd.message_id,
            cmd.actor_user_id,
            cmd.actor_party_id,
        );
        self.message_repo.mark_read(&read).await?;

        self.publisher
            .publish(crate::ports::MessageEvent::MessageRead {
                message_id: cmd.message_id,
                user_id: cmd.actor_user_id,
                party_id: cmd.actor_party_id,
                read_at: read.read_at,
            })
            .await?;

        Ok(read)
    }
}
