use crate::errors::ApplicationError;
use crate::notifications::dto::UnreadCountResult;
use domain::repositories::NotificationRepository;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct GetUnreadCount {
    repo: Arc<dyn NotificationRepository>,
}

impl GetUnreadCount {
    pub fn new(repo: Arc<dyn NotificationRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        user_id: Uuid,
        party_id: Option<Uuid>,
    ) -> Result<UnreadCountResult, ApplicationError> {
        let count = self
            .repo
            .count_unread_for_recipient(Some(user_id), party_id)
            .await?;
        Ok(UnreadCountResult { count })
    }
}
