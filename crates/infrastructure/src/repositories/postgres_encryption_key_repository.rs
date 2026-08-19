use async_trait::async_trait;
use domain::entities::EncryptionKey;
use domain::errors::DomainError;
use domain::repositories::EncryptionKeyRepository;
use sqlx::{Error as SqlxError, PgPool};
use uuid::Uuid;

pub struct PostgresEncryptionKeyRepository {
    pool: PgPool,
}

impl PostgresEncryptionKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EncryptionKeyRepository for PostgresEncryptionKeyRepository {
    async fn create(&self, key: &EncryptionKey) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO encryption_keys (id, key_name, key_bytes, is_active, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            key.id,
            key.key_name,
            key.key_bytes,
            key.is_active,
            key.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_active(&self) -> Result<Option<EncryptionKey>, DomainError> {
        let row = sqlx::query_as!(
            EncryptionKeyRow,
            r#"
            SELECT id, key_name, key_bytes, is_active, created_at
            FROM encryption_keys
            WHERE is_active = true
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_key))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<EncryptionKey>, DomainError> {
        let row = sqlx::query_as!(
            EncryptionKeyRow,
            r#"
            SELECT id, key_name, key_bytes, is_active, created_at
            FROM encryption_keys
            WHERE key_name = $1
            "#,
            name
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_key))
    }

    async fn deactivate_all(&self) -> Result<u64, DomainError> {
        let result = sqlx::query!(
            r#"
            UPDATE encryption_keys
            SET is_active = false
            WHERE is_active = true
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(result.rows_affected())
    }

    async fn activate(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE encryption_keys
            SET is_active = true
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
struct EncryptionKeyRow {
    id: Uuid,
    key_name: String,
    key_bytes: String,
    is_active: bool,
    created_at: time::OffsetDateTime,
}

fn build_key(row: EncryptionKeyRow) -> EncryptionKey {
    EncryptionKey {
        id: row.id,
        key_name: row.key_name,
        key_bytes: row.key_bytes,
        is_active: row.is_active,
        created_at: row.created_at,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
