use crate::errors::ApplicationError;
use crate::notifications::dto::UpdateNotificationBody;
use crate::ports::{NotificationEvent, NotificationRealtimePublisher};
use domain::repositories::NotificationRepository;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone)]
pub struct MarkNotificationRead {
    repo: Arc<dyn NotificationRepository>,
    publisher: Arc<dyn NotificationRealtimePublisher>,
}

impl MarkNotificationRead {
    pub fn new(
        repo: Arc<dyn NotificationRepository>,
        publisher: Arc<dyn NotificationRealtimePublisher>,
    ) -> Self {
        Self { repo, publisher }
    }

    pub async fn execute(
        &self,
        id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        body: UpdateNotificationBody,
    ) -> Result<(), ApplicationError> {
        let now = OffsetDateTime::now_utc();
        let mut changed = false;

        if body.is_read == Some(true) {
            changed = self.repo.mark_read(id, user_id, party_id, now).await?;
            if changed {
                self.publisher
                    .publish(NotificationEvent::NotificationRead {
                        notification_id: id,
                        user_id,
                    })
                    .await?;
                let count = self
                    .repo
                    .count_unread_for_recipient(Some(user_id), party_id)
                    .await?;
                self.publisher
                    .publish(NotificationEvent::UnreadCountChanged {
                        user_id: Some(user_id),
                        party_id,
                        count,
                    })
                    .await?;
            }
        }

        if body.is_actioned == Some(true) {
            self.repo.mark_actioned(id, user_id, party_id, now).await?;
            changed = true;
        }

        if !changed {
            let notification = self
                .repo
                .find_by_id(id)
                .await?
                .ok_or(ApplicationError::NotificationNotFound)?;
            let owned = notification.user_id == Some(user_id) || notification.party_id == party_id;
            if !owned {
                return Err(ApplicationError::Forbidden);
            }
        }

        Ok(())
    }
}
