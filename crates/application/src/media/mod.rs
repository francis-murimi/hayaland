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
            || (upload.owner_party_id.is_some() && upload.owner_party_id == cmd.actor_party_id);
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::errors::DomainError;
    use std::sync::Mutex;

    struct FakeMediaRepo {
        uploads: Mutex<Vec<MediaUpload>>,
        soft_deleted: Mutex<Vec<Uuid>>,
    }

    impl FakeMediaRepo {
        fn new() -> Self {
            Self {
                uploads: Mutex::new(Vec::new()),
                soft_deleted: Mutex::new(Vec::new()),
            }
        }

        fn with(upload: MediaUpload) -> Self {
            Self {
                uploads: Mutex::new(vec![upload]),
                soft_deleted: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl MediaRepository for FakeMediaRepo {
        async fn create(&self, upload: &MediaUpload) -> Result<(), DomainError> {
            self.uploads.lock().unwrap().push(upload.clone());
            Ok(())
        }

        async fn find_by_id(
            &self,
            id: Uuid,
            _include_deleted: bool,
        ) -> Result<Option<MediaUpload>, DomainError> {
            Ok(self
                .uploads
                .lock()
                .unwrap()
                .iter()
                .find(|u| u.id == id)
                .cloned())
        }

        async fn list(&self, filters: &MediaFilters) -> Result<MediaListResult, DomainError> {
            let uploads = self.uploads.lock().unwrap();
            let items: Vec<_> = uploads
                .iter()
                .filter(|u| {
                    filters
                        .owner_user_id
                        .map(|o| u.owner_user_id == o)
                        .unwrap_or(true)
                })
                .cloned()
                .collect();
            Ok(MediaListResult {
                total: items.len() as i64,
                items,
            })
        }

        async fn soft_delete(&self, id: Uuid) -> Result<bool, DomainError> {
            self.soft_deleted.lock().unwrap().push(id);
            Ok(true)
        }

        async fn delete_permanently(&self, _id: Uuid) -> Result<bool, DomainError> {
            Ok(true)
        }
    }

    struct FakeStorage {
        deleted: Mutex<Vec<String>>,
        fail_delete: bool,
    }

    impl FakeStorage {
        fn new() -> Self {
            Self {
                deleted: Mutex::new(Vec::new()),
                fail_delete: false,
            }
        }

        fn failing_delete() -> Self {
            Self {
                deleted: Mutex::new(Vec::new()),
                fail_delete: true,
            }
        }
    }

    #[async_trait]
    impl MediaStorage for FakeStorage {
        async fn store(
            &self,
            _content: &[u8],
            extension: Option<&str>,
        ) -> Result<String, ApplicationError> {
            Ok(match extension {
                Some(ext) => format!("2026/08/file.{ext}"),
                None => "2026/08/file".to_string(),
            })
        }

        async fn delete(&self, path: &str) -> Result<(), ApplicationError> {
            if self.fail_delete {
                return Err(ApplicationError::MediaStorageFailed {
                    message: "disk gone".to_string(),
                });
            }
            self.deleted.lock().unwrap().push(path.to_string());
            Ok(())
        }

        async fn public_url(&self, path: &str) -> Result<String, ApplicationError> {
            Ok(format!("/uploads/{path}"))
        }
    }

    fn upload_cmd(content_type: &str) -> UploadMediaCommand {
        UploadMediaCommand {
            actor_user_id: Uuid::now_v7(),
            actor_party_id: None,
            purpose: "MESSAGE_ATTACHMENT".to_string(),
            related_entity_type: None,
            related_entity_id: None,
            content_type: content_type.to_string(),
            original_filename: "photo.png".to_string(),
            size_bytes: 4,
            is_public: None,
        }
    }

    fn stored_upload(owner: Uuid) -> MediaUpload {
        MediaUpload::new(
            Uuid::now_v7(),
            owner,
            None,
            MediaPurpose::MessageAttachment,
            None,
            None,
            "photo.png".to_string(),
            "file.png".to_string(),
            "2026/08/file.png".to_string(),
            "image/png".to_string(),
            4,
            "deadbeef".to_string(),
            false,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn upload_media_happy_path() {
        let repo = Arc::new(FakeMediaRepo::new());
        let storage = Arc::new(FakeStorage::new());
        let uc = UploadMedia::new(repo.clone(), storage, 1024, vec!["image/*".to_string()]);
        let result = uc
            .execute(upload_cmd("image/png"), b"data".to_vec())
            .await
            .unwrap();
        assert_eq!(result.stored_filename, "file.png");
        assert_eq!(result.url, "/uploads/2026/08/file.png");
        assert_eq!(result.size_bytes, 4);
        assert_eq!(result.sha256.len(), 64);
        assert!(!result.is_public);
        assert_eq!(repo.uploads.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn upload_media_rejects_oversize() {
        let uc = UploadMedia::new(
            Arc::new(FakeMediaRepo::new()),
            Arc::new(FakeStorage::new()),
            2,
            vec![],
        );
        let err = uc
            .execute(upload_cmd("image/png"), b"data".to_vec())
            .await
            .unwrap_err();
        assert!(matches!(err, ApplicationError::MediaTooLarge));
    }

    #[tokio::test]
    async fn upload_media_rejects_disallowed_content_type() {
        let uc = UploadMedia::new(
            Arc::new(FakeMediaRepo::new()),
            Arc::new(FakeStorage::new()),
            1024,
            vec!["image/*".to_string()],
        );
        let err = uc
            .execute(upload_cmd("application/x-evil"), b"data".to_vec())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ApplicationError::InvalidMediaContentType { .. }
        ));
    }

    #[tokio::test]
    async fn upload_media_rejects_unknown_purpose() {
        let uc = UploadMedia::new(
            Arc::new(FakeMediaRepo::new()),
            Arc::new(FakeStorage::new()),
            1024,
            vec![],
        );
        let mut cmd = upload_cmd("image/png");
        cmd.purpose = "NOPE".to_string();
        let err = uc.execute(cmd, b"data".to_vec()).await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn upload_media_rejects_unknown_related_entity_type() {
        let uc = UploadMedia::new(
            Arc::new(FakeMediaRepo::new()),
            Arc::new(FakeStorage::new()),
            1024,
            vec![],
        );
        let mut cmd = upload_cmd("image/png");
        cmd.related_entity_type = Some("NOPE".to_string());
        let err = uc.execute(cmd, b"data".to_vec()).await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn upload_media_without_extension_and_exact_type_match() {
        let repo = Arc::new(FakeMediaRepo::new());
        let uc = UploadMedia::new(
            repo,
            Arc::new(FakeStorage::new()),
            1024,
            vec!["text/plain".to_string()],
        );
        let mut cmd = upload_cmd("text/plain");
        cmd.original_filename = "notes".to_string();
        cmd.is_public = Some(true);
        let result = uc.execute(cmd, b"data".to_vec()).await.unwrap();
        assert_eq!(result.stored_filename, "file");
        assert!(result.is_public);
    }

    #[tokio::test]
    async fn list_media_returns_mapped_items() {
        let owner = Uuid::now_v7();
        let repo = Arc::new(FakeMediaRepo::with(stored_upload(owner)));
        let storage = Arc::new(FakeStorage::new());
        let uc = ListMedia::new(repo);
        let result = uc
            .execute(
                ListMediaCommand {
                    owner_user_id: Some(owner),
                    owner_party_id: None,
                    related_entity_type: None,
                    related_entity_id: None,
                    include_deleted: None,
                    limit: None,
                    offset: None,
                },
                storage,
            )
            .await
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].url, "/uploads/2026/08/file.png");
        assert_eq!(result.items[0].purpose, "MESSAGE_ATTACHMENT");
    }

    #[tokio::test]
    async fn delete_media_owner_succeeds() {
        let owner = Uuid::now_v7();
        let upload = stored_upload(owner);
        let id = upload.id;
        let repo = Arc::new(FakeMediaRepo::with(upload));
        let storage = Arc::new(FakeStorage::new());
        let uc = DeleteMedia::new(repo.clone(), storage.clone());
        uc.execute(
            id,
            DeleteMediaCommand {
                actor_user_id: owner,
                actor_party_id: None,
                is_admin: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(repo.soft_deleted.lock().unwrap().as_slice(), &[id]);
        assert_eq!(
            storage.deleted.lock().unwrap().as_slice(),
            &["2026/08/file.png".to_string()]
        );
    }

    #[tokio::test]
    async fn delete_media_forbidden_for_non_owner() {
        let owner = Uuid::now_v7();
        let upload = stored_upload(owner);
        let id = upload.id;
        let uc = DeleteMedia::new(
            Arc::new(FakeMediaRepo::with(upload)),
            Arc::new(FakeStorage::new()),
        );
        let err = uc
            .execute(
                id,
                DeleteMediaCommand {
                    actor_user_id: Uuid::now_v7(),
                    actor_party_id: None,
                    is_admin: false,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, ApplicationError::Forbidden));
    }

    #[tokio::test]
    async fn delete_media_admin_can_delete_others() {
        let upload = stored_upload(Uuid::now_v7());
        let id = upload.id;
        let uc = DeleteMedia::new(
            Arc::new(FakeMediaRepo::with(upload)),
            Arc::new(FakeStorage::new()),
        );
        uc.execute(
            id,
            DeleteMediaCommand {
                actor_user_id: Uuid::now_v7(),
                actor_party_id: None,
                is_admin: true,
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn delete_media_not_found() {
        let uc = DeleteMedia::new(Arc::new(FakeMediaRepo::new()), Arc::new(FakeStorage::new()));
        let err = uc
            .execute(
                Uuid::now_v7(),
                DeleteMediaCommand {
                    actor_user_id: Uuid::now_v7(),
                    actor_party_id: None,
                    is_admin: true,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, ApplicationError::MediaNotFound));
    }

    #[tokio::test]
    async fn delete_media_tolerates_storage_failure() {
        let owner = Uuid::now_v7();
        let upload = stored_upload(owner);
        let id = upload.id;
        let uc = DeleteMedia::new(
            Arc::new(FakeMediaRepo::with(upload)),
            Arc::new(FakeStorage::failing_delete()),
        );
        uc.execute(
            id,
            DeleteMediaCommand {
                actor_user_id: owner,
                actor_party_id: None,
                is_admin: false,
            },
        )
        .await
        .unwrap();
    }
}
