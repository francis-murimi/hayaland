use async_trait::async_trait;
use domain::entities::MatchSuggestionAuditLogEntry;
use domain::errors::DomainError;
use domain::repositories::MatchSuggestionAuditLogRepository;
use sqlx::{Error as SqlxError, PgPool};
use uuid::Uuid;

pub struct PostgresMatchSuggestionAuditLogRepository {
    pool: PgPool,
}

impl PostgresMatchSuggestionAuditLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MatchSuggestionAuditLogRepository for PostgresMatchSuggestionAuditLogRepository {
    async fn create(&self, entry: &MatchSuggestionAuditLogEntry) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO match_suggestion_audit_log (
                id, admin_user_id, action_type, match_suggestion_id,
                party_id, before_snapshot, after_snapshot, reason, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7::jsonb, $8, $9)
            "#,
            entry.id,
            entry.admin_user_id,
            entry.action_type,
            entry.match_suggestion_id,
            entry.party_id,
            entry.before_snapshot,
            entry.after_snapshot,
            entry.reason,
            entry.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list_by_match_suggestion(
        &self,
        match_suggestion_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MatchSuggestionAuditLogEntry>, DomainError> {
        let rows = sqlx::query_as!(
            MatchSuggestionAuditLogRow,
            r#"
            SELECT
                id,
                admin_user_id,
                action_type,
                match_suggestion_id,
                party_id,
                before_snapshot as "before_snapshot!: serde_json::Value",
                after_snapshot as "after_snapshot!: serde_json::Value",
                reason,
                created_at
            FROM match_suggestion_audit_log
            WHERE match_suggestion_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            match_suggestion_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_entry).collect())
    }

    async fn list_by_admin(
        &self,
        admin_user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MatchSuggestionAuditLogEntry>, DomainError> {
        let rows = sqlx::query_as!(
            MatchSuggestionAuditLogRow,
            r#"
            SELECT
                id,
                admin_user_id,
                action_type,
                match_suggestion_id,
                party_id,
                before_snapshot as "before_snapshot!: serde_json::Value",
                after_snapshot as "after_snapshot!: serde_json::Value",
                reason,
                created_at
            FROM match_suggestion_audit_log
            WHERE admin_user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            admin_user_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_entry).collect())
    }

    async fn list_by_party(
        &self,
        party_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MatchSuggestionAuditLogEntry>, DomainError> {
        let rows = sqlx::query_as!(
            MatchSuggestionAuditLogRow,
            r#"
            SELECT
                id,
                admin_user_id,
                action_type,
                match_suggestion_id,
                party_id,
                before_snapshot as "before_snapshot!: serde_json::Value",
                after_snapshot as "after_snapshot!: serde_json::Value",
                reason,
                created_at
            FROM match_suggestion_audit_log
            WHERE party_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            party_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_entry).collect())
    }
}

#[derive(sqlx::FromRow)]
struct MatchSuggestionAuditLogRow {
    id: Uuid,
    admin_user_id: Option<Uuid>,
    action_type: String,
    match_suggestion_id: Option<Uuid>,
    party_id: Option<Uuid>,
    before_snapshot: Option<serde_json::Value>,
    after_snapshot: Option<serde_json::Value>,
    reason: Option<String>,
    created_at: time::OffsetDateTime,
}

fn build_entry(row: MatchSuggestionAuditLogRow) -> MatchSuggestionAuditLogEntry {
    MatchSuggestionAuditLogEntry {
        id: row.id,
        admin_user_id: row.admin_user_id,
        action_type: row.action_type,
        match_suggestion_id: row.match_suggestion_id,
        party_id: row.party_id,
        before_snapshot: row.before_snapshot,
        after_snapshot: row.after_snapshot,
        reason: row.reason,
        created_at: row.created_at,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
