pub mod dto;

use crate::errors::ApplicationError;
use crate::media::dto::{
    DeleteMediaCommand, ListMediaCommand, MediaListDto, UploadMediaCommand, UploadMediaResult,
};
use crate::ports::MediaStorage;
use domain::entities::{MediaPurpose, MediaRelatedEntityType, MediaUpload};
use domain::repositories::{MediaFilters, MediaListResult, MediaRepository};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::{info, instrument, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct UploadMedia {
    media_repo: Arc<dyn MediaRepository>,
    storage: Arc<dyn MediaStorage>,
    max_size_bytes: usize,
    allowed_content_types: Vec<String>,
}

impl UploadMedia {
    pub fn new(
        media_repo: Arc<dyn MediaRepository>,
        storage: Arc<dyn MediaStorage>,
        max_size_bytes: usize,
        allowed_content_types: Vec<String>,
    ) -> Self {
        Self {
            media_repo,
            storage,
            max_size_bytes,
            allowed_content_types,
        }
    }

    #[instrument(skip(self, content))]
    pub async fn execute(
        &self,
        cmd: UploadMediaCommand,
        content: Vec<u8>,
    ) -> Result<UploadMediaResult, ApplicationError> {
        let size = content.len();
        if size > self.max_size_bytes {
            return Err(ApplicationError::MediaTooLarge);
        }
        if !self.is_content_type_allowed(&cmd.content_type) {
            return Err(ApplicationError::InvalidMediaContentType {
                message: format!("content type {} is not allowed", cmd.content_type),
            });
        }

        let purpose =
            MediaPurpose::try_from(cmd.purpose.as_str()).map_err(ApplicationError::from)?;
        let related_entity_type = cmd
            .related_entity_type
            .as_deref()
            .map(MediaRelatedEntityType::try_from)
            .transpose()
            .map_err(ApplicationError::from)?;

        let sha256 = hex::encode(Sha256::digest(&content));
        let extension = cmd.original_filename.rsplit_once('.').map(|(_, ext)| ext);
        let storage_path = self.storage.store(&content, extension).await?;
        let stored_filename = storage_path
            .rsplit_once('/')
            .map(|(_, name)| name.to_string())
            .unwrap_or_else(|| storage_path.clone());

        let upload = MediaUpload::new(
            Uuid::now_v7(),
            cmd.actor_user_id,
            cmd.actor_party_id,
            purpose,
            related_entity_type,
            cmd.related_entity_id,
            cmd.original_filename,
            stored_filename,
            storage_path.clone(),
            cmd.content_type,
            size as i32,
            sha256,
            cmd.is_public.unwrap_or(false),
        )
        .map_err(ApplicationError::from)?;

        self.media_repo.create(&upload).await?;

        let url = self.storage.public_url(&storage_path).await?;
        info!(media_id = %upload.id, actor = %cmd.actor_user_id, "uploaded media");
        Ok(map_upload(upload, url))
    }

    fn is_content_type_allowed(&self, content_type: &str) -> bool {
        if self.allowed_content_types.is_empty() {
            return true;
        }
        self.allowed_content_types.iter().any(|allowed| {
            if allowed.ends_with("/*") {
                let prefix = allowed.trim_end_matches("/*");
                content_type.starts_with(prefix)
            } else {
                content_type == allowed
            }
        })
    }
}

#[derive(Clone)]
pub struct ListMedia {
    media_repo: Arc<dyn MediaRepository>,
}

impl ListMedia {
    pub fn new(media_repo: Arc<dyn MediaRepository>) -> Self {
        Self { media_repo }
    }

    pub async fn execute(
        &self,
        cmd: ListMediaCommand,
        storage: Arc<dyn MediaStorage>,
    ) -> Result<MediaListDto, ApplicationError> {
        let filters = MediaFilters {
            owner_user_id: cmd.owner_user_id,
            owner_party_id: cmd.owner_party_id,
            related_entity_type: cmd.related_entity_type,
            related_entity_id: cmd.related_entity_id,
            include_deleted: cmd.include_deleted.unwrap_or(false),
            limit: cmd.limit.unwrap_or(20).clamp(1, 100),
            offset: cmd.offset.unwrap_or(0).max(0),
        };

        let MediaListResult { items, total } = self.media_repo.list(&filters).await?;
        let mut results = Vec::with_capacity(items.len());
        for upload in items {
            let url = storage.public_url(&upload.storage_path).await?;
            results.push(map_upload(upload, url));
        }
        Ok(MediaListDto {
            items: results,
            total,
        })
    }
}

#[derive(Clone)]
pub struct DeleteMedia {
    media_repo: Arc<dyn MediaRepository>,
    storage: Arc<dyn MediaStorage>,
}

impl DeleteMedia {
    pub fn new(media_repo: Arc<dyn MediaRepository>, storage: Arc<dyn MediaStorage>) -> Self {
        Self {
            media_repo,
            storage,
        }
    }

    pub async fn execute(&self, id: Uuid, cmd: DeleteMediaCommand) -> Result<(), ApplicationError> {
        let upload = self
            .media_repo
            .find_by_id(id, false)
            .await?
            .ok_or(ApplicationError::MediaNotFound)?;

        let is_owner = upload.owner_user_id == cmd.actor_user_id
            || upload.owner_party_id == cmd.actor_party_id;
        if !is_owner && !cmd.is_admin {
            return Err(ApplicationError::Forbidden);
        }

        self.media_repo.soft_delete(id).await?;
        if let Err(err) = self.storage.delete(&upload.storage_path).await {
            warn!(media_id = %id, error = %err, "failed to delete stored media file");
        }
        info!(media_id = %id, "deleted media upload");
        Ok(())
    }
}

fn map_upload(upload: MediaUpload, url: String) -> UploadMediaResult {
    UploadMediaResult {
        id: upload.id,
        owner_user_id: upload.owner_user_id,
        owner_party_id: upload.owner_party_id,
        purpose: upload.purpose.as_str().to_string(),
        related_entity_type: upload.related_entity_type.map(|t| t.as_str().to_string()),
        related_entity_id: upload.related_entity_id,
        original_filename: upload.original_filename,
        stored_filename: upload.stored_filename,
        storage_path: upload.storage_path,
        content_type: upload.content_type,
        size_bytes: upload.size_bytes,
        sha256: upload.sha256,
        is_public: upload.is_public,
        url,
        created_at: upload.created_at,
    }
}
