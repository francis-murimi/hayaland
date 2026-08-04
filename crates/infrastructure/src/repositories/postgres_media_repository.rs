use async_trait::async_trait;
use domain::entities::{MediaPurpose, MediaRelatedEntityType, MediaUpload};
use domain::errors::DomainError;
use domain::repositories::{MediaFilters, MediaListResult, MediaRepository};
use sqlx::{Error as SqlxError, PgPool};
use uuid::Uuid;

pub struct PostgresMediaRepository {
    pool: PgPool,
}

impl PostgresMediaRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MediaRepository for PostgresMediaRepository {
    async fn create(&self, upload: &MediaUpload) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO media_uploads (
                id, owner_user_id, owner_party_id, purpose, related_entity_type, related_entity_id,
                original_filename, stored_filename, storage_path, content_type, size_bytes, sha256,
                is_public, created_at, deleted_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            "#,
            upload.id,
            upload.owner_user_id,
            upload.owner_party_id,
            upload.purpose.as_str(),
            upload
                .related_entity_type
                .as_ref()
                .map(|t| t.as_str().to_string()),
            upload.related_entity_id,
            upload.original_filename,
            upload.stored_filename,
            upload.storage_path,
            upload.content_type,
            upload.size_bytes,
            upload.sha256,
            upload.is_public,
            upload.created_at,
            upload.deleted_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_id(
        &self,
        id: Uuid,
        include_deleted: bool,
    ) -> Result<Option<MediaUpload>, DomainError> {
        let row = sqlx::query_as!(
            MediaUploadRow,
            r#"
            SELECT
                id,
                owner_user_id,
                owner_party_id,
                purpose,
                related_entity_type,
                related_entity_id,
                original_filename,
                stored_filename,
                storage_path,
                content_type,
                size_bytes,
                sha256,
                is_public,
                created_at,
                deleted_at
            FROM media_uploads
            WHERE id = $1
              AND ($2 OR deleted_at IS NULL)
            "#,
            id,
            include_deleted
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_media_upload))
    }

    async fn list(&self, filters: &MediaFilters) -> Result<MediaListResult, DomainError> {
        let rows = sqlx::query_as!(
            MediaUploadRow,
            r#"
            SELECT
                id,
                owner_user_id,
                owner_party_id,
                purpose,
                related_entity_type,
                related_entity_id,
                original_filename,
                stored_filename,
                storage_path,
                content_type,
                size_bytes,
                sha256,
                is_public,
                created_at,
                deleted_at
            FROM media_uploads
            WHERE ($1::UUID IS NULL OR owner_user_id = $1)
              AND ($2::UUID IS NULL OR owner_party_id = $2)
              AND ($3::TEXT IS NULL OR related_entity_type = $3)
              AND ($4::UUID IS NULL OR related_entity_id = $4)
              AND ($5 OR deleted_at IS NULL)
            ORDER BY created_at DESC
            LIMIT $6 OFFSET $7
            "#,
            filters.owner_user_id,
            filters.owner_party_id,
            filters.related_entity_type,
            filters.related_entity_id,
            filters.include_deleted,
            filters.limit,
            filters.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM media_uploads
            WHERE ($1::UUID IS NULL OR owner_user_id = $1)
              AND ($2::UUID IS NULL OR owner_party_id = $2)
              AND ($3::TEXT IS NULL OR related_entity_type = $3)
              AND ($4::UUID IS NULL OR related_entity_id = $4)
              AND ($5 OR deleted_at IS NULL)
            "#,
            filters.owner_user_id,
            filters.owner_party_id,
            filters.related_entity_type,
            filters.related_entity_id,
            filters.include_deleted
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(MediaListResult {
            items: rows.into_iter().map(build_media_upload).collect(),
            total,
        })
    }

    async fn soft_delete(&self, id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query!(
            r#"
            UPDATE media_uploads
            SET deleted_at = now()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(result.rows_affected() > 0)
    }

    async fn delete_permanently(&self, id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM media_uploads
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(result.rows_affected() > 0)
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}

#[derive(Debug, sqlx::FromRow)]
struct MediaUploadRow {
    id: Uuid,
    owner_user_id: Uuid,
    owner_party_id: Option<Uuid>,
    purpose: String,
    related_entity_type: Option<String>,
    related_entity_id: Option<Uuid>,
    original_filename: String,
    stored_filename: String,
    storage_path: String,
    content_type: String,
    size_bytes: i32,
    sha256: String,
    is_public: bool,
    created_at: time::OffsetDateTime,
    deleted_at: Option<time::OffsetDateTime>,
}

fn build_media_upload(row: MediaUploadRow) -> MediaUpload {
    MediaUpload {
        id: row.id,
        owner_user_id: row.owner_user_id,
        owner_party_id: row.owner_party_id,
        purpose: MediaPurpose::try_from(row.purpose.as_str()).expect("stored purpose is valid"),
        related_entity_type: row
            .related_entity_type
            .as_deref()
            .map(MediaRelatedEntityType::try_from)
            .transpose()
            .expect("stored related_entity_type is valid"),
        related_entity_id: row.related_entity_id,
        original_filename: row.original_filename,
        stored_filename: row.stored_filename,
        storage_path: row.storage_path,
        content_type: row.content_type,
        size_bytes: row.size_bytes,
        sha256: row.sha256,
        is_public: row.is_public,
        created_at: row.created_at,
        deleted_at: row.deleted_at,
    }
}
