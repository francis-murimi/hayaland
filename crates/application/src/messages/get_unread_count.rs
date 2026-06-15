use crate::errors::ApplicationError;
use domain::repositories::MessageRepository;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct GetUnreadCount {
    message_repo: Arc<dyn MessageRepository>,
}

impl GetUnreadCount {
    pub fn new(message_repo: Arc<dyn MessageRepository>) -> Self {
        Self { message_repo }
    }

    pub async fn execute(
        &self,
        actor_user_id: Uuid,
        actor_party_id: Option<Uuid>,
    ) -> Result<i64, ApplicationError> {
        self.message_repo
            .unread_count_for_user(actor_user_id, actor_party_id)
            .await
            .map_err(Into::into)
    }
}
