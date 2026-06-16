use async_trait::async_trait;
use domain::entities::{NotificationChannel, NotificationTemplate, NotificationType};
use domain::errors::DomainError;
use domain::repositories::{NotificationTemplateRepository, Pagination};
use sqlx::{Error as SqlxError, PgPool};
use uuid::Uuid;

pub struct PostgresNotificationTemplateRepository {
    pool: PgPool,
}

impl PostgresNotificationTemplateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NotificationTemplateRepository for PostgresNotificationTemplateRepository {
    async fn create(&self, template: &NotificationTemplate) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO notification_templates (
                id, name, notification_type, channel, locale,
                subject_template, body_template, variables_schema,
                is_active, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            template.id,
            template.name,
            template.notification_type.as_str(),
            template.channel.as_str(),
            template.locale,
            template.subject_template,
            template.body_template,
            template.variables_schema,
            template.is_active,
            template.created_at,
            template.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update(&self, template: &NotificationTemplate) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE notification_templates
            SET name = $1,
                notification_type = $2,
                channel = $3,
                locale = $4,
                subject_template = $5,
                body_template = $6,
                variables_schema = $7,
                is_active = $8,
                updated_at = $9
            WHERE id = $10
            "#,
            template.name,
            template.notification_type.as_str(),
            template.channel.as_str(),
            template.locale,
            template.subject_template,
            template.body_template,
            template.variables_schema,
            template.is_active,
            template.updated_at,
            template.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<NotificationTemplate>, DomainError> {
        let row = sqlx::query_as!(
            TemplateRow,
            r#"
            SELECT id, name, notification_type, channel, locale,
                   subject_template, body_template, variables_schema as "variables_schema!: sqlx::types::Json<serde_json::Value>",
                   is_active, created_at, updated_at
            FROM notification_templates
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_template))
    }

    async fn find_active(
        &self,
        notification_type: NotificationType,
        channel: NotificationChannel,
        locale: &str,
    ) -> Result<Option<NotificationTemplate>, DomainError> {
        let row = sqlx::query_as!(
            TemplateRow,
            r#"
            SELECT id, name, notification_type, channel, locale,
                   subject_template, body_template, variables_schema as "variables_schema!: sqlx::types::Json<serde_json::Value>",
                   is_active, created_at, updated_at
            FROM notification_templates
            WHERE notification_type = $1
              AND channel = $2
              AND locale = $3
              AND is_active = true
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
            notification_type.as_str(),
            channel.as_str(),
            locale
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_template))
    }

    async fn list(&self, pagination: Pagination) -> Result<Vec<NotificationTemplate>, DomainError> {
        let rows = sqlx::query_as!(
            TemplateRow,
            r#"
            SELECT id, name, notification_type, channel, locale,
                   subject_template, body_template, variables_schema as "variables_schema!: sqlx::types::Json<serde_json::Value>",
                   is_active, created_at, updated_at
            FROM notification_templates
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            pagination.limit,
            pagination.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_template).collect())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            DELETE FROM notification_templates
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct TemplateRow {
    id: Uuid,
    name: String,
    notification_type: String,
    channel: String,
    locale: String,
    subject_template: String,
    body_template: String,
    variables_schema: sqlx::types::Json<serde_json::Value>,
    is_active: bool,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn build_template(row: TemplateRow) -> NotificationTemplate {
    let mut template = NotificationTemplate::new(
        row.id,
        row.name,
        NotificationType::try_from(row.notification_type.as_str())
            .expect("stored notification type is valid"),
        NotificationChannel::try_from(row.channel.as_str()).expect("stored channel is valid"),
        row.locale,
        row.subject_template,
        row.body_template,
        row.variables_schema.0,
    )
    .expect("stored template is valid");

    template.is_active = row.is_active;
    template.created_at = row.created_at;
    template.updated_at = row.updated_at;

    template
}

fn map_err(err: SqlxError) -> DomainError {
    if let SqlxError::Database(db_err) = &err {
        match db_err.constraint() {
            Some("notification_templates_pkey") => {
                return DomainError::RepositoryError("template already exists".to_string())
            }
            Some("notification_templates_name_key") => {
                return DomainError::DuplicateNotificationTemplate
            }
            Some("notification_templates_notification_type_channel_locale_key") => {
                return DomainError::DuplicateNotificationTemplate
            }
            _ => {}
        }
    }
    DomainError::RepositoryError(err.to_string())
}
