use crate::entities::notification_preference::NotificationPreference;
use crate::entities::{
    Notification, NotificationChannel, NotificationStatus, NotificationTemplate, NotificationType,
};
use crate::errors::DomainError;
use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

/// Filtering options for listing notifications.
#[derive(Debug, Clone, Default)]
pub struct NotificationFilters {
    pub notification_type: Option<NotificationType>,
    pub is_read: Option<bool>,
    pub is_actioned: Option<bool>,
    pub priority: Option<crate::entities::NotificationPriority>,
}

/// Simple cursor-based pagination.
#[derive(Debug, Clone, Default)]
pub struct Pagination {
    pub limit: i64,
    pub offset: i64,
}

/// Result of a notification list query.
#[derive(Debug, Clone)]
pub struct NotificationListResult {
    pub items: Vec<Notification>,
    pub total: i64,
    pub unread_count: i64,
}

/// Outcome of a single channel delivery attempt.
#[derive(Debug, Clone)]
pub enum DeliveryResult {
    Sent,
    Delivered,
    Failed { message: String },
}

/// Outbound port for persisting and retrieving notifications.
#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn create(&self, notification: &Notification) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, DomainError>;
    async fn list_for_recipient(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
        filters: NotificationFilters,
        pagination: Pagination,
    ) -> Result<NotificationListResult, DomainError>;
    async fn count_unread_for_recipient(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
    ) -> Result<i64, DomainError>;
    async fn mark_read(
        &self,
        id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        read_at: OffsetDateTime,
    ) -> Result<bool, DomainError>;
    async fn mark_all_read(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
        before: Option<OffsetDateTime>,
        notification_type: Option<NotificationType>,
    ) -> Result<u64, DomainError>;
    async fn mark_actioned(
        &self,
        id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        actioned_at: OffsetDateTime,
    ) -> Result<bool, DomainError>;
    async fn delete(
        &self,
        id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
    ) -> Result<bool, DomainError>;
    async fn update_status(&self, id: Uuid, status: NotificationStatus) -> Result<(), DomainError>;
    async fn record_delivery(
        &self,
        notification_id: Uuid,
        channel: NotificationChannel,
        result: DeliveryResult,
    ) -> Result<(), DomainError>;
    /// Poll pending notifications for the background worker.
    async fn list_pending(
        &self,
        batch_size: usize,
        older_than: Option<OffsetDateTime>,
    ) -> Result<Vec<Notification>, DomainError>;
}

/// Outbound port for notification preferences.
#[async_trait]
pub trait NotificationPreferenceRepository: Send + Sync {
    async fn get(&self, user_id: Uuid) -> Result<NotificationPreference, DomainError>;
    async fn save(&self, preference: &NotificationPreference) -> Result<(), DomainError>;
}

/// Outbound port for notification templates.
#[async_trait]
pub trait NotificationTemplateRepository: Send + Sync {
    async fn create(&self, template: &NotificationTemplate) -> Result<(), DomainError>;
    async fn update(&self, template: &NotificationTemplate) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<NotificationTemplate>, DomainError>;
    async fn find_active(
        &self,
        notification_type: NotificationType,
        channel: NotificationChannel,
        locale: &str,
    ) -> Result<Option<NotificationTemplate>, DomainError>;
    async fn list(&self, pagination: Pagination) -> Result<Vec<NotificationTemplate>, DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}
