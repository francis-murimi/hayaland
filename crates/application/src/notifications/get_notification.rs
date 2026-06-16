use crate::errors::ApplicationError;
use crate::notifications::dto::NotificationResult;
use domain::repositories::NotificationRepository;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct GetNotification {
    repo: Arc<dyn NotificationRepository>,
}

impl GetNotification {
    pub fn new(repo: Arc<dyn NotificationRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
    ) -> Result<NotificationResult, ApplicationError> {
        let notification = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotificationNotFound)?;

        let owned = notification.user_id == Some(user_id)
            || notification
                .party_id
                .is_some_and(|pid| Some(pid) == party_id);

        if !owned {
            return Err(ApplicationError::Forbidden);
        }

        Ok(NotificationResult::from(&notification))
    }
}
