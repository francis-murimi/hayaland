use async_trait::async_trait;
use domain::entities::EmailVerification;
use domain::errors::DomainError;
use domain::repositories::EmailVerificationRepository;
use sqlx::{Error as SqlxError, PgPool};
use uuid::Uuid;

pub struct PostgresEmailVerificationRepository {
    pool: PgPool,
}

impl PostgresEmailVerificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EmailVerificationRepository for PostgresEmailVerificationRepository {
    async fn save(&self, verification: &EmailVerification) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO email_verifications (token, user_id, expires_at, used)
            VALUES ($1, $2, $3, $4)
            "#,
            verification.token,
            verification.user_id,
            verification.expires_at,
            verification.used,
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_token(&self, token: &str) -> Result<Option<EmailVerification>, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT token, user_id, expires_at, used
            FROM email_verifications
            WHERE token = $1
            "#,
            token
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(|r| EmailVerification {
            token: r.token,
            user_id: r.user_id,
            expires_at: r.expires_at,
            used: r.used,
        }))
    }

    async fn mark_used(&self, token: &str) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE email_verifications
            SET used = true
            WHERE token = $1
            "#,
            token
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn invalidate_unused_for_user(&self, user_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE email_verifications
            SET used = true
            WHERE user_id = $1 AND used = false
            "#,
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
