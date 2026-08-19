use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// A persisted AES-256-GCM encryption key that can be rotated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncryptionKey {
    pub id: Uuid,
    pub key_name: String,
    /// Base64-encoded 32-byte key.
    pub key_bytes: String,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
}

impl EncryptionKey {
    pub fn new(id: Uuid, key_name: impl Into<String>, key_bytes: impl Into<String>) -> Self {
        Self {
            id,
            key_name: key_name.into(),
            key_bytes: key_bytes.into(),
            is_active: true,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}
