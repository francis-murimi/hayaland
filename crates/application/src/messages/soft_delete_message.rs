use crate::errors::ApplicationError;
use crate::messages::dto::{to_message_result, MessageResult, SoftDeleteMessageCommand};
use crate::ports::{EncryptionService, RealtimePublisher};
use domain::repositories::MessageRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct SoftDeleteMessage {
    message_repo: Arc<dyn MessageRepository>,
    encryption: Arc<dyn EncryptionService>,
    publisher: Arc<dyn RealtimePublisher>,
}

impl SoftDeleteMessage {
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
        cmd: SoftDeleteMessageCommand,
    ) -> Result<MessageResult, ApplicationError> {
        let mut message = self
            .message_repo
            .find_message_by_id(cmd.message_id)
            .await?
            .ok_or(ApplicationError::MessageNotFound)?;

        if !message.can_be_deleted_by(cmd.actor_user_id, cmd.is_admin) {
            return Err(ApplicationError::CannotDeleteMessage);
        }

        let placeholder = self.encryption.encrypt("").await?;
        message.soft_delete(placeholder);
        self.message_repo.update_message(&message).await?;

        self.publisher
            .publish(crate::ports::MessageEvent::MessageDeleted {
                message_id: message.id,
                conversation_id: message.conversation_id,
            })
            .await?;

        to_message_result(message, &self.encryption).await
    }
}
