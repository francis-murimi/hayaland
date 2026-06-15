use crate::errors::ApplicationError;
use crate::messages::dto::{to_message_result, EditMessageCommand, MessageResult};
use crate::ports::{EncryptionService, RealtimePublisher};
use domain::entities::MessageType;
use domain::repositories::MessageRepository;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct EditMessage {
    message_repo: Arc<dyn MessageRepository>,
    encryption: Arc<dyn EncryptionService>,
    publisher: Arc<dyn RealtimePublisher>,
}

impl EditMessage {
    pub fn new(
        message_repo: Arc<dyn MessageRepository>,
        encryption: Arc<dyn EncryptionService>,
        publisher: Arc<dyn RealtimePublisher>,
    ) -> Self {
        Self {
            message_repo,
            encryption,
            publisher,
        }
    }

    pub async fn execute(
        &self,
        cmd: EditMessageCommand,
    ) -> Result<MessageResult, ApplicationError> {
        let mut message = self
            .message_repo
            .find_message_by_id(cmd.message_id)
            .await?
            .ok_or(ApplicationError::MessageNotFound)?;

        if !message.can_be_edited_by(cmd.actor_user_id, cmd.is_admin) {
            return Err(ApplicationError::CannotEditMessage);
        }

        if matches!(
            message.message_type,
            MessageType::System | MessageType::AdminBroadcast
        ) && !cmd.is_admin
        {
            return Err(ApplicationError::CannotEditMessage);
        }

        let encrypted = self.encryption.encrypt(&cmd.content).await?;
        message.edit(encrypted);
        self.message_repo.update_message(&message).await?;

        self.publisher
            .publish(crate::ports::MessageEvent::MessageUpdated {
                message_id: message.id,
                conversation_id: message.conversation_id,
                content: cmd.content.clone(),
                edited_at: message.edited_at.unwrap_or_else(OffsetDateTime::now_utc),
            })
            .await?;

        to_message_result(message, &self.encryption).await
    }
}
