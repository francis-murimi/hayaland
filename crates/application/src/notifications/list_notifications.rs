use crate::errors::ApplicationError;
use crate::notifications::dto::{
    NotificationListQuery, NotificationListResultDto, NotificationResult,
};
use domain::repositories::{NotificationFilters, NotificationRepository, Pagination};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ListNotifications {
    repo: Arc<dyn NotificationRepository>,
}

impl ListNotifications {
    pub fn new(repo: Arc<dyn NotificationRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        user_id: Uuid,
        party_id: Option<Uuid>,
        query: NotificationListQuery,
    ) -> Result<NotificationListResultDto, ApplicationError> {
        let filters = NotificationFilters {
            notification_type: query.notification_type,
            is_read: query.is_read,
            is_actioned: query.is_actioned,
            priority: query.priority,
        };
        let pagination = Pagination {
            limit: query.limit.unwrap_or(20).clamp(1, 100),
            offset: query.offset.unwrap_or(0).max(0),
        };

        let result = self
            .repo
            .list_for_recipient(Some(user_id), party_id, filters, pagination)
            .await?;

        Ok(NotificationListResultDto {
            data: result.items.iter().map(NotificationResult::from).collect(),
            unread_count: result.unread_count,
            total: result.total,
        })
    }
}
