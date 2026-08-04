use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Why a media file was uploaded and which domain it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaPurpose {
    MessageAttachment,
    DisputeEvidence,
    VerificationEvidence,
    AgreementDocument,
    Other,
}

impl MediaPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaPurpose::MessageAttachment => "MESSAGE_ATTACHMENT",
            MediaPurpose::DisputeEvidence => "DISPUTE_EVIDENCE",
            MediaPurpose::VerificationEvidence => "VERIFICATION_EVIDENCE",
            MediaPurpose::AgreementDocument => "AGREEMENT_DOCUMENT",
            MediaPurpose::Other => "OTHER",
        }
    }
}

impl TryFrom<&str> for MediaPurpose {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "MESSAGE_ATTACHMENT" => Ok(MediaPurpose::MessageAttachment),
            "DISPUTE_EVIDENCE" => Ok(MediaPurpose::DisputeEvidence),
            "VERIFICATION_EVIDENCE" => Ok(MediaPurpose::VerificationEvidence),
            "AGREEMENT_DOCUMENT" => Ok(MediaPurpose::AgreementDocument),
            "OTHER" => Ok(MediaPurpose::Other),
            _ => Err(DomainError::InvalidMediaPurpose {
                message: format!("unknown media purpose: {value}"),
            }),
        }
    }
}

/// The business entity a media file is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaRelatedEntityType {
    Message,
    Dispute,
    Verification,
    Agreement,
    Deal,
    Party,
    CatalogItem,
}

impl MediaRelatedEntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaRelatedEntityType::Message => "MESSAGE",
            MediaRelatedEntityType::Dispute => "DISPUTE",
            MediaRelatedEntityType::Verification => "VERIFICATION",
            MediaRelatedEntityType::Agreement => "AGREEMENT",
            MediaRelatedEntityType::Deal => "DEAL",
            MediaRelatedEntityType::Party => "PARTY",
            MediaRelatedEntityType::CatalogItem => "CATALOG_ITEM",
        }
    }
}

impl TryFrom<&str> for MediaRelatedEntityType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "MESSAGE" => Ok(MediaRelatedEntityType::Message),
            "DISPUTE" => Ok(MediaRelatedEntityType::Dispute),
            "VERIFICATION" => Ok(MediaRelatedEntityType::Verification),
            "AGREEMENT" => Ok(MediaRelatedEntityType::Agreement),
            "DEAL" => Ok(MediaRelatedEntityType::Deal),
            "PARTY" => Ok(MediaRelatedEntityType::Party),
            "CATALOG_ITEM" => Ok(MediaRelatedEntityType::CatalogItem),
            _ => Err(DomainError::InvalidMediaRelatedEntityType {
                message: format!("unknown media related entity type: {value}"),
            }),
        }
    }
}

/// A persisted media upload tracked by the platform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaUpload {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub owner_party_id: Option<Uuid>,
    pub purpose: MediaPurpose,
    pub related_entity_type: Option<MediaRelatedEntityType>,
    pub related_entity_id: Option<Uuid>,
    pub original_filename: String,
    pub stored_filename: String,
    pub storage_path: String,
    pub content_type: String,
    pub size_bytes: i32,
    pub sha256: String,
    pub is_public: bool,
    pub created_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl MediaUpload {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        owner_user_id: Uuid,
        owner_party_id: Option<Uuid>,
        purpose: MediaPurpose,
        related_entity_type: Option<MediaRelatedEntityType>,
        related_entity_id: Option<Uuid>,
        original_filename: String,
        stored_filename: String,
        storage_path: String,
        content_type: String,
        size_bytes: i32,
        sha256: String,
        is_public: bool,
    ) -> Result<Self, DomainError> {
        if original_filename.trim().is_empty() {
            return Err(DomainError::InvalidMediaContentType {
                message: "original filename cannot be empty".to_string(),
            });
        }
        if content_type.trim().is_empty() {
            return Err(DomainError::InvalidMediaContentType {
                message: "content type cannot be empty".to_string(),
            });
        }
        if size_bytes < 0 {
            return Err(DomainError::InvalidMediaSize {
                message: "size bytes cannot be negative".to_string(),
            });
        }
        if sha256.trim().is_empty() {
            return Err(DomainError::InvalidMediaContentType {
                message: "sha256 cannot be empty".to_string(),
            });
        }
        Ok(Self {
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
            created_at: OffsetDateTime::now_utc(),
            deleted_at: None,
        })
    }

    pub fn mark_deleted(&mut self) {
        self.deleted_at = Some(OffsetDateTime::now_utc());
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_upload() -> MediaUpload {
        MediaUpload::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            None,
            MediaPurpose::Other,
            None,
            None,
            "file.txt".to_string(),
            "stored.txt".to_string(),
            "uploads/stored.txt".to_string(),
            "text/plain".to_string(),
            100,
            "deadbeef".to_string(),
            false,
        )
        .unwrap()
    }

    #[test]
    fn media_upload_rejects_empty_filename() {
        let result = MediaUpload::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            None,
            MediaPurpose::Other,
            None,
            None,
            "   ".to_string(),
            "stored.txt".to_string(),
            "uploads/stored.txt".to_string(),
            "text/plain".to_string(),
            100,
            "deadbeef".to_string(),
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn media_upload_rejects_negative_size() {
        let result = MediaUpload::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            None,
            MediaPurpose::Other,
            None,
            None,
            "file.txt".to_string(),
            "stored.txt".to_string(),
            "uploads/stored.txt".to_string(),
            "text/plain".to_string(),
            -1,
            "deadbeef".to_string(),
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn mark_deleted_sets_deleted_at() {
        let mut upload = valid_upload();
        assert!(!upload.is_deleted());
        upload.mark_deleted();
        assert!(upload.is_deleted());
    }

    #[test]
    fn purpose_round_trip() {
        for p in [
            MediaPurpose::MessageAttachment,
            MediaPurpose::DisputeEvidence,
            MediaPurpose::VerificationEvidence,
            MediaPurpose::AgreementDocument,
            MediaPurpose::Other,
        ] {
            assert_eq!(MediaPurpose::try_from(p.as_str()).unwrap(), p);
        }
    }
}
