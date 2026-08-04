use async_trait::async_trait;
use domain::entities::{AdminAction, AdminActionTargetType, AdminActionType};
use domain::errors::DomainError;
use domain::repositories::{AuditLogFilters, AuditLogListResult, AuditLogRepository};
use sqlx::{Error as SqlxError, PgPool};
use uuid::Uuid;

pub struct PostgresAuditLogRepository {
    pool: PgPool,
}

impl PostgresAuditLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditLogRepository for PostgresAuditLogRepository {
    async fn create(&self, action: &AdminAction) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO admin_actions (
                id, admin_user_id, action_type, target_type, target_id,
                before_snapshot, after_snapshot, reason, ip_address, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            action.id,
            action.admin_user_id,
            action.action_type.as_str(),
            action.target_type.as_str(),
            action.target_id,
            action.before_snapshot,
            action.after_snapshot,
            action.reason,
            action.ip_address,
            action.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list(&self, filters: &AuditLogFilters) -> Result<AuditLogListResult, DomainError> {
        let action_type = filters.action_type.as_ref().map(|t| t.as_str().to_string());
        let target_type = filters.target_type.as_ref().map(|t| t.as_str().to_string());

        let rows = sqlx::query_as!(
            AdminActionRow,
            r#"
            SELECT
                id,
                admin_user_id,
                action_type,
                target_type,
                target_id,
                before_snapshot,
                after_snapshot,
                reason,
                ip_address,
                created_at
            FROM admin_actions
            WHERE ($1::UUID IS NULL OR admin_user_id = $1)
              AND ($2::TEXT IS NULL OR action_type = $2)
              AND ($3::TEXT IS NULL OR target_type = $3)
              AND ($4::UUID IS NULL OR target_id = $4)
              AND ($5::TIMESTAMPTZ IS NULL OR created_at >= $5)
              AND ($6::TIMESTAMPTZ IS NULL OR created_at <= $6)
            ORDER BY created_at DESC
            LIMIT $7 OFFSET $8
            "#,
            filters.admin_user_id,
            action_type,
            target_type,
            filters.target_id,
            filters.from,
            filters.to,
            filters.limit,
            filters.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM admin_actions
            WHERE ($1::UUID IS NULL OR admin_user_id = $1)
              AND ($2::TEXT IS NULL OR action_type = $2)
              AND ($3::TEXT IS NULL OR target_type = $3)
              AND ($4::UUID IS NULL OR target_id = $4)
              AND ($5::TIMESTAMPTZ IS NULL OR created_at >= $5)
              AND ($6::TIMESTAMPTZ IS NULL OR created_at <= $6)
            "#,
            filters.admin_user_id,
            action_type,
            target_type,
            filters.target_id,
            filters.from,
            filters.to
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(AuditLogListResult {
            items: rows.into_iter().map(build_admin_action).collect(),
            total,
        })
    }

    async fn list_for_target(
        &self,
        target_type: AdminActionTargetType,
        target_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<AuditLogListResult, DomainError> {
        let rows = sqlx::query_as!(
            AdminActionRow,
            r#"
            SELECT
                id,
                admin_user_id,
                action_type,
                target_type,
                target_id,
                before_snapshot,
                after_snapshot,
                reason,
                ip_address,
                created_at
            FROM admin_actions
            WHERE target_type = $1 AND target_id = $2
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
            target_type.as_str(),
            target_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM admin_actions
            WHERE target_type = $1 AND target_id = $2
            "#,
            target_type.as_str(),
            target_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(AuditLogListResult {
            items: rows.into_iter().map(build_admin_action).collect(),
            total,
        })
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}

#[derive(Debug, sqlx::FromRow)]
struct AdminActionRow {
    id: Uuid,
    admin_user_id: Uuid,
    action_type: String,
    target_type: String,
    target_id: Uuid,
    before_snapshot: Option<serde_json::Value>,
    after_snapshot: Option<serde_json::Value>,
    reason: Option<String>,
    ip_address: Option<String>,
    created_at: time::OffsetDateTime,
}

fn build_admin_action(row: AdminActionRow) -> AdminAction {
    AdminAction {
        id: row.id,
        admin_user_id: row.admin_user_id,
        action_type: AdminActionType::try_from(row.action_type.as_str())
            .expect("stored action_type is valid"),
        target_type: AdminActionTargetType::try_from(row.target_type.as_str())
            .expect("stored target_type is valid"),
        target_id: row.target_id,
        before_snapshot: row.before_snapshot,
        after_snapshot: row.after_snapshot,
        reason: row.reason,
        ip_address: row.ip_address,
        created_at: row.created_at,
    }
}
