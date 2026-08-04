use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct UploadMediaCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub purpose: String,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<Uuid>,
    pub content_type: String,
    pub original_filename: String,
    pub size_bytes: i64,
    pub is_public: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UploadMediaResult {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub owner_party_id: Option<Uuid>,
    pub purpose: String,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<Uuid>,
    pub original_filename: String,
    pub stored_filename: String,
    pub storage_path: String,
    pub content_type: String,
    pub size_bytes: i32,
    pub sha256: String,
    pub is_public: bool,
    pub url: String,
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ListMediaCommand {
    pub owner_user_id: Option<Uuid>,
    pub owner_party_id: Option<Uuid>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<Uuid>,
    pub include_deleted: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaListDto {
    pub items: Vec<UploadMediaResult>,
    pub total: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteMediaCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub is_admin: bool,
}
