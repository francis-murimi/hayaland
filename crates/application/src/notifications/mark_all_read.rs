use crate::errors::ApplicationError;
use crate::notifications::dto::MarkAllReadBody;
use crate::ports::{NotificationEvent, NotificationRealtimePublisher};
use domain::repositories::NotificationRepository;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct MarkAllNotificationsRead {
    repo: Arc<dyn NotificationRepository>,
    publisher: Arc<dyn NotificationRealtimePublisher>,
}

impl MarkAllNotificationsRead {
    pub fn new(
        repo: Arc<dyn NotificationRepository>,
        publisher: Arc<dyn NotificationRealtimePublisher>,
    ) -> Self {
        Self { repo, publisher }
    }

    pub async fn execute(
        &self,
        user_id: Uuid,
        party_id: Option<Uuid>,
        body: MarkAllReadBody,
    ) -> Result<u64, ApplicationError> {
        let count = self
            .repo
            .mark_all_read(
                Some(user_id),
                party_id,
                body.before_date,
                body.notification_type,
            )
            .await?;

        if count > 0 {
            let unread = self
                .repo
                .count_unread_for_recipient(Some(user_id), party_id)
                .await?;
            self.publisher
                .publish(NotificationEvent::UnreadCountChanged {
                    user_id: Some(user_id),
                    party_id,
                    count: unread,
                })
                .await?;
        }

        Ok(count)
    }
}
