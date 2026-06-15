use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateVerificationRequest {
    #[serde(rename = "verificationType")]
    pub verification_type: String,
    #[serde(rename = "evidenceUrls")]
    pub evidence_urls: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct RejectVerificationRequest {
    pub reason: String,
    #[serde(rename = "reviewNotes")]
    pub review_notes: Option<String>,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct RevokeVerificationRequest {
    pub reason: String,
    #[serde(rename = "reviewNotes")]
    pub review_notes: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResponse {
    pub id: Uuid,
    pub party_id: Uuid,
    pub requested_by_user_id: Uuid,
    pub reviewed_by_user_id: Option<Uuid>,
    pub verification_type: String,
    pub status: String,
    pub points: i32,
    pub evidence_urls: Vec<String>,
    pub provider_reference: Option<String>,
    pub rejection_reason: Option<String>,
    pub review_notes: Option<String>,
    pub requested_at: OffsetDateTime,
    pub reviewed_at: Option<OffsetDateTime>,
    pub expires_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<application::verifications::dto::VerificationResult> for VerificationResponse {
    fn from(v: application::verifications::dto::VerificationResult) -> Self {
        Self {
            id: v.id,
            party_id: v.party_id,
            requested_by_user_id: v.requested_by_user_id,
            reviewed_by_user_id: v.reviewed_by_user_id,
            verification_type: v.verification_type,
            status: v.status,
            points: v.points,
            evidence_urls: v.evidence_urls,
            provider_reference: v.provider_reference,
            rejection_reason: v.rejection_reason,
            review_notes: v.review_notes,
            requested_at: v.requested_at,
            reviewed_at: v.reviewed_at,
            expires_at: v.expires_at,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationsResponse {
    pub verifications: Vec<VerificationResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

impl From<application::verifications::dto::VerificationListResult> for VerificationsResponse {
    fn from(r: application::verifications::dto::VerificationListResult) -> Self {
        Self {
            verifications: r.verifications.into_iter().map(Into::into).collect(),
            total: r.total,
            limit: r.limit,
            offset: r.offset,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationStatusResponse {
    pub party_id: Uuid,
    pub verification_status: String,
    pub verification_level: i32,
    pub effective_points: i64,
    pub pending_count: i64,
    pub approved_count: i64,
    pub rejected_count: i64,
    pub revoked_count: i64,
    pub expired_count: i64,
    pub next_level_points: i32,
}

impl From<application::verifications::dto::VerificationStatusResult>
    for VerificationStatusResponse
{
    fn from(r: application::verifications::dto::VerificationStatusResult) -> Self {
        Self {
            party_id: r.party_id,
            verification_status: r.verification_status,
            verification_level: r.verification_level,
            effective_points: r.effective_points,
            pending_count: r.pending_count,
            approved_count: r.approved_count,
            rejected_count: r.rejected_count,
            revoked_count: r.revoked_count,
            expired_count: r.expired_count,
            next_level_points: r.next_level_points,
        }
    }
}
