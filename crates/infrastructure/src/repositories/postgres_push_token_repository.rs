use async_trait::async_trait;
use domain::entities::PushToken;
use domain::errors::DomainError;
use domain::repositories::PushTokenRepository;
use sqlx::{Error as SqlxError, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresPushTokenRepository {
    pool: PgPool,
}

impl PostgresPushTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PushTokenRepository for PostgresPushTokenRepository {
    async fn upsert(&self, token: &PushToken) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO user_push_tokens (
                id, user_id, device_token, provider, device_type, created_at, last_used_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (user_id, device_token)
            DO UPDATE SET
                provider = EXCLUDED.provider,
                device_type = EXCLUDED.device_type,
                last_used_at = EXCLUDED.last_used_at
            "#,
            token.id,
            token.user_id,
            token.device_token,
            token.provider,
            token.device_type,
            token.created_at,
            token.last_used_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<PushToken>, DomainError> {
        let rows = sqlx::query_as!(
            PushTokenRow,
            r#"
            SELECT id, user_id, device_token, provider, device_type, created_at, last_used_at
            FROM user_push_tokens
            WHERE user_id = $1
            ORDER BY last_used_at DESC NULLS LAST, created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_token).collect())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PushToken>, DomainError> {
        let row = sqlx::query_as!(
            PushTokenRow,
            r#"
            SELECT id, user_id, device_token, provider, device_type, created_at, last_used_at
            FROM user_push_tokens
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_token))
    }

    async fn delete(&self, id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM user_push_tokens
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(result.rows_affected() > 0)
    }

    async fn delete_by_user(&self, user_id: Uuid) -> Result<u64, DomainError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM user_push_tokens
            WHERE user_id = $1
            "#,
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(result.rows_affected())
    }

    async fn touch(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE user_push_tokens
            SET last_used_at = $1
            WHERE id = $2
            "#,
            OffsetDateTime::now_utc(),
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct PushTokenRow {
    id: Uuid,
    user_id: Uuid,
    device_token: String,
    provider: String,
    device_type: Option<String>,
    created_at: time::OffsetDateTime,
    last_used_at: Option<time::OffsetDateTime>,
}

fn build_token(row: PushTokenRow) -> PushToken {
    PushToken {
        id: row.id,
        user_id: row.user_id,
        device_token: row.device_token,
        provider: row.provider,
        device_type: row.device_type,
        created_at: row.created_at,
        last_used_at: row.last_used_at,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
