use async_trait::async_trait;
use domain::entities::{
    ActionType, Notification, NotificationAction, NotificationChannel, NotificationPriority,
    NotificationStatus, NotificationType,
};
use domain::errors::DomainError;
use domain::repositories::{
    DeliveryResult, NotificationFilters, NotificationListResult, NotificationRepository, Pagination,
};
use sqlx::{Error as SqlxError, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresNotificationRepository {
    pool: PgPool,
}

impl PostgresNotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NotificationRepository for PostgresNotificationRepository {
    async fn create(&self, notification: &Notification) -> Result<(), DomainError> {
        let channels: Vec<String> = notification
            .channels
            .iter()
            .map(|c| c.as_str().to_string())
            .collect();
        let actions = serde_json::to_value(&notification.actions)
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        let metadata = notification.metadata.clone();

        sqlx::query!(
            r#"
            INSERT INTO notifications (
                id, user_id, party_id, notification_type, title, body, channels,
                priority, status, read_at, actioned_at, expires_at, action_url,
                actions, related_entity_type, related_entity_id, metadata,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
            "#,
            notification.id,
            notification.user_id,
            notification.party_id,
            notification.notification_type.as_str(),
            notification.title,
            notification.body,
            &channels,
            notification.priority.as_str(),
            notification.status.as_str(),
            notification.read_at,
            notification.actioned_at,
            notification.expires_at,
            notification.action_url,
            actions,
            notification.related_entity_type,
            notification.related_entity_id,
            metadata,
            notification.created_at,
            notification.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, DomainError> {
        let row = sqlx::query_as!(
            NotificationRow,
            r#"
            SELECT id, user_id, party_id, notification_type, title, body, channels,
                   priority, status, read_at, actioned_at, expires_at, action_url,
                   actions as "actions!: sqlx::types::Json<Vec<NotificationActionRow>>",
                   related_entity_type, related_entity_id, metadata as "metadata!: sqlx::types::Json<serde_json::Value>",
                   created_at, updated_at
            FROM notifications
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_notification))
    }

    async fn list_for_recipient(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
        filters: NotificationFilters,
        pagination: Pagination,
    ) -> Result<NotificationListResult, DomainError> {
        let notification_type = filters
            .notification_type
            .as_ref()
            .map(|t| t.as_str().to_string());
        let priority = filters.priority.as_ref().map(|p| p.as_str().to_string());

        let rows = sqlx::query_as!(
            NotificationRow,
            r#"
            SELECT id, user_id, party_id, notification_type, title, body, channels,
                   priority, status, read_at, actioned_at, expires_at, action_url,
                   actions as "actions!: sqlx::types::Json<Vec<NotificationActionRow>>",
                   related_entity_type, related_entity_id, metadata as "metadata!: sqlx::types::Json<serde_json::Value>",
                   created_at, updated_at
            FROM notifications
            WHERE (
                (user_id IS NOT DISTINCT FROM $1)
                OR (party_id IS NOT DISTINCT FROM $2)
            )
            AND ($3::TEXT IS NULL OR notification_type = $3)
            AND ($4::BOOLEAN IS NULL OR (read_at IS NOT NULL) = $4)
            AND ($5::BOOLEAN IS NULL OR (actioned_at IS NOT NULL) = $5)
            AND ($6::TEXT IS NULL OR priority = $6)
            ORDER BY created_at DESC
            LIMIT $7 OFFSET $8
            "#,
            user_id,
            party_id,
            notification_type,
            filters.is_read,
            filters.is_actioned,
            priority,
            pagination.limit,
            pagination.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM notifications
            WHERE (
                (user_id IS NOT DISTINCT FROM $1)
                OR (party_id IS NOT DISTINCT FROM $2)
            )
            AND ($3::TEXT IS NULL OR notification_type = $3)
            AND ($4::BOOLEAN IS NULL OR (read_at IS NOT NULL) = $4)
            AND ($5::BOOLEAN IS NULL OR (actioned_at IS NOT NULL) = $5)
            AND ($6::TEXT IS NULL OR priority = $6)
            "#,
            user_id,
            party_id,
            notification_type,
            filters.is_read,
            filters.is_actioned,
            priority
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let unread_count = self.count_unread_for_recipient(user_id, party_id).await?;

        Ok(NotificationListResult {
            items: rows.into_iter().map(build_notification).collect(),
            total,
            unread_count,
        })
    }

    async fn count_unread_for_recipient(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
    ) -> Result<i64, DomainError> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM notifications
            WHERE (
                (user_id IS NOT DISTINCT FROM $1)
                OR (party_id IS NOT DISTINCT FROM $2)
            )
            AND read_at IS NULL
            "#,
            user_id,
            party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(count)
    }

    async fn mark_read(
        &self,
        id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        read_at: OffsetDateTime,
    ) -> Result<bool, DomainError> {
        let result = sqlx::query!(
            r#"
            UPDATE notifications
            SET read_at = $1, status = 'DELIVERED', updated_at = $1
            WHERE id = $2
              AND (
                  (user_id = $3)
                  OR (party_id IS NOT DISTINCT FROM $4)
              )
              AND read_at IS NULL
            "#,
            read_at,
            id,
            user_id,
            party_id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(result.rows_affected() > 0)
    }

    async fn mark_all_read(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
        before: Option<OffsetDateTime>,
        notification_type: Option<NotificationType>,
    ) -> Result<u64, DomainError> {
        let notification_type_str = notification_type.as_ref().map(|t| t.as_str().to_string());
        let now = OffsetDateTime::now_utc();

        let result = sqlx::query!(
            r#"
            UPDATE notifications
            SET read_at = $1, status = 'DELIVERED', updated_at = $1
            WHERE (
                (user_id IS NOT DISTINCT FROM $2)
                OR (party_id IS NOT DISTINCT FROM $3)
            )
            AND read_at IS NULL
            AND ($4::TIMESTAMPTZ IS NULL OR created_at <= $4)
            AND ($5::TEXT IS NULL OR notification_type = $5)
            "#,
            now,
            user_id,
            party_id,
            before,
            notification_type_str
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(result.rows_affected())
    }

    async fn mark_actioned(
        &self,
        id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        actioned_at: OffsetDateTime,
    ) -> Result<bool, DomainError> {
        let result = sqlx::query!(
            r#"
            UPDATE notifications
            SET actioned_at = $1, updated_at = $1
            WHERE id = $2
              AND (
                  (user_id = $3)
                  OR (party_id IS NOT DISTINCT FROM $4)
              )
              AND actioned_at IS NULL
            "#,
            actioned_at,
            id,
            user_id,
            party_id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(result.rows_affected() > 0)
    }

    async fn delete(
        &self,
        id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
    ) -> Result<bool, DomainError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM notifications
            WHERE id = $1
              AND (
                  (user_id = $2)
                  OR (party_id IS NOT DISTINCT FROM $3)
              )
            "#,
            id,
            user_id,
            party_id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(result.rows_affected() > 0)
    }

    async fn update_status(&self, id: Uuid, status: NotificationStatus) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE notifications
            SET status = $1, updated_at = now()
            WHERE id = $2
            "#,
            status.as_str(),
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn record_delivery(
        &self,
        notification_id: Uuid,
        channel: NotificationChannel,
        result: DeliveryResult,
    ) -> Result<(), DomainError> {
        let (status, delivered_at, error_message) = match result {
            DeliveryResult::Sent => ("SENT", None::<OffsetDateTime>, None::<String>),
            DeliveryResult::Delivered => {
                ("DELIVERED", Some(OffsetDateTime::now_utc()), None::<String>)
            }
            DeliveryResult::Failed { message } => ("FAILED", None::<OffsetDateTime>, Some(message)),
        };

        sqlx::query!(
            r#"
            INSERT INTO notification_delivery_records (
                id, notification_id, channel, status, attempted_at, delivered_at, error_message
            )
            VALUES ($1, $2, $3, $4, now(), $5, $6)
            "#,
            Uuid::now_v7(),
            notification_id,
            channel.as_str(),
            status,
            delivered_at,
            error_message
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list_pending(
        &self,
        batch_size: usize,
        older_than: Option<OffsetDateTime>,
    ) -> Result<Vec<Notification>, DomainError> {
        let rows = sqlx::query_as!(
            NotificationRow,
            r#"
            SELECT id, user_id, party_id, notification_type, title, body, channels,
                   priority, status, read_at, actioned_at, expires_at, action_url,
                   actions as "actions!: sqlx::types::Json<Vec<NotificationActionRow>>",
                   related_entity_type, related_entity_id, metadata as "metadata!: sqlx::types::Json<serde_json::Value>",
                   created_at, updated_at
            FROM notifications
            WHERE status = 'PENDING'
              AND ($1::TIMESTAMPTZ IS NULL OR created_at <= $1)
            ORDER BY created_at ASC
            LIMIT $2
            "#,
            older_than,
            batch_size as i64
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_notification).collect())
    }
}

#[derive(sqlx::FromRow)]
struct NotificationRow {
    id: Uuid,
    user_id: Option<Uuid>,
    party_id: Option<Uuid>,
    notification_type: String,
    title: String,
    body: String,
    channels: Vec<String>,
    priority: String,
    status: String,
    read_at: Option<OffsetDateTime>,
    actioned_at: Option<OffsetDateTime>,
    expires_at: Option<OffsetDateTime>,
    action_url: Option<String>,
    actions: sqlx::types::Json<Vec<NotificationActionRow>>,
    related_entity_type: Option<String>,
    related_entity_id: Option<Uuid>,
    metadata: sqlx::types::Json<serde_json::Value>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct NotificationActionRow {
    label: String,
    action_type: String,
    url: Option<String>,
    method: Option<String>,
}

fn build_notification(row: NotificationRow) -> Notification {
    let actions: Vec<NotificationAction> = row
        .actions
        .0
        .into_iter()
        .map(|a| NotificationAction {
            label: a.label,
            action_type: ActionType::try_from(a.action_type.as_str())
                .expect("stored action type is valid"),
            url: a.url,
            method: a.method,
        })
        .collect();

    let mut notification = Notification::new(
        row.id,
        row.user_id,
        row.party_id,
        NotificationType::try_from(row.notification_type.as_str())
            .expect("stored notification type is valid"),
        row.title,
        row.body,
        NotificationPriority::try_from(row.priority.as_str()).expect("stored priority is valid"),
        row.action_url,
        actions,
        row.related_entity_type,
        row.related_entity_id,
        row.metadata.0,
        row.expires_at,
    )
    .expect("stored notification is valid");

    notification.status =
        NotificationStatus::try_from(row.status.as_str()).expect("stored status is valid");
    notification.channels = row
        .channels
        .iter()
        .map(|c| NotificationChannel::try_from(c.as_str()).expect("stored channel is valid"))
        .collect();
    notification.read_at = row.read_at;
    notification.actioned_at = row.actioned_at;
    notification.created_at = row.created_at;
    notification.updated_at = row.updated_at;

    notification
}

fn map_err(err: SqlxError) -> DomainError {
    if let SqlxError::Database(db_err) = &err {
        match db_err.constraint() {
            Some("notifications_pkey") => {
                return DomainError::RepositoryError("notification already exists".to_string())
            }
            Some("notification_templates_name_key") => {
                return DomainError::DuplicateNotificationTemplate
            }
            Some("notification_templates_notification_type_channel_locale_key") => {
                return DomainError::DuplicateNotificationTemplate
            }
            Some("notifications_user_id_fkey") => {
                return DomainError::RepositoryError("user not found".to_string())
            }
            Some("notifications_party_id_fkey") => return DomainError::PartyNotFound,
            _ => {}
        }
    }
    DomainError::RepositoryError(err.to_string())
}
