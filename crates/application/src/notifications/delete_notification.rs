use crate::errors::ApplicationError;
use domain::repositories::NotificationRepository;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct DeleteNotification {
    repo: Arc<dyn NotificationRepository>,
}

impl DeleteNotification {
    pub fn new(repo: Arc<dyn NotificationRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
    ) -> Result<(), ApplicationError> {
        let deleted = self.repo.delete(id, user_id, party_id).await?;
        if !deleted {
            // Distinguish not-found vs forbidden by reading first.
            let exists = self.repo.find_by_id(id).await?.is_some();
            return Err(if exists {
                ApplicationError::Forbidden
            } else {
                ApplicationError::NotificationNotFound
            });
        }
        Ok(())
    }
}
