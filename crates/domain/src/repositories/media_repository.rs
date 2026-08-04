use crate::entities::MediaUpload;
use crate::errors::DomainError;
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct MediaFilters {
    pub owner_user_id: Option<Uuid>,
    pub owner_party_id: Option<Uuid>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<Uuid>,
    pub include_deleted: bool,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct MediaListResult {
    pub items: Vec<MediaUpload>,
    pub total: i64,
}

/// Outbound port for persisting and retrieving media uploads.
#[async_trait]
pub trait MediaRepository: Send + Sync {
    /// Save a new media upload record.
    async fn create(&self, upload: &MediaUpload) -> Result<(), DomainError>;

    /// Find a media upload by id, optionally including soft-deleted rows.
    async fn find_by_id(
        &self,
        id: Uuid,
        include_deleted: bool,
    ) -> Result<Option<MediaUpload>, DomainError>;

    /// List media uploads matching the filters.
    async fn list(&self, filters: &MediaFilters) -> Result<MediaListResult, DomainError>;

    /// Soft-delete a media upload by id.
    async fn soft_delete(&self, id: Uuid) -> Result<bool, DomainError>;

    /// Permanently delete a media upload by id.
    async fn delete_permanently(&self, id: Uuid) -> Result<bool, DomainError>;
}
