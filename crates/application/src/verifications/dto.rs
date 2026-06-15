use domain::entities::{PartyVerification, VerificationStatus};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Command to submit a verification request.
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitVerificationCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub target_party_id: Uuid,
    pub is_admin: bool,
    pub verification_type: String,
    pub evidence_urls: Vec<String>,
    pub notes: Option<String>,
}

/// Command to approve a pending verification.
#[derive(Debug, Clone, Deserialize)]
pub struct ApproveVerificationCommand {
    pub actor_user_id: Uuid,
    pub verification_id: Uuid,
    pub review_notes: Option<String>,
}

/// Command to reject a pending verification.
#[derive(Debug, Clone, Deserialize)]
pub struct RejectVerificationCommand {
    pub actor_user_id: Uuid,
    pub verification_id: Uuid,
    pub reason: String,
    pub review_notes: Option<String>,
}

/// Command to revoke an approved verification.
#[derive(Debug, Clone, Deserialize)]
pub struct RevokeVerificationCommand {
    pub actor_user_id: Uuid,
    pub verification_id: Uuid,
    pub reason: String,
    pub review_notes: Option<String>,
}

/// Query to list a party's verifications.
#[derive(Debug, Clone, Deserialize)]
pub struct ListPartyVerificationsQuery {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
}

/// Query to get verification status summary for a party.
#[derive(Debug, Clone, Deserialize)]
pub struct GetVerificationStatusQuery {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
}

/// Filters for the admin verification queue.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdminVerificationListQuery {
    pub status: Option<String>,
    pub verification_type: Option<String>,
    pub party_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

/// Single verification as returned by application use cases.
#[derive(Debug, Clone, Serialize)]
pub struct VerificationResult {
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

impl From<PartyVerification> for VerificationResult {
    fn from(v: PartyVerification) -> Self {
        Self {
            id: v.id,
            party_id: v.party_id,
            requested_by_user_id: v.requested_by_user_id,
            reviewed_by_user_id: v.reviewed_by_user_id,
            verification_type: v.verification_type.as_str().to_string(),
            status: v.status.as_str().to_string(),
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

/// Paginated list of verifications.
#[derive(Debug, Clone, Serialize)]
pub struct VerificationListResult {
    pub verifications: Vec<VerificationResult>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Summary of a party's verification state.
#[derive(Debug, Clone, Serialize)]
pub struct VerificationStatusResult {
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

/// Determine the next verification level threshold from the current effective points.
pub fn next_level_points(effective_points: i64) -> i32 {
    match effective_points {
        0 => 10,
        10..=24 => 25,
        25..=54 => 55,
        55..=79 => 80,
        80..=99 => 100,
        _ => 0,
    }
}

/// Map effective approved points and pending count to the high-level party `VerificationStatus`.
pub fn party_verification_status_for_points(
    effective_points: i64,
    pending_count: i64,
) -> VerificationStatus {
    if effective_points > 0 {
        VerificationStatus::Verified
    } else if pending_count > 0 {
        VerificationStatus::Pending
    } else {
        VerificationStatus::Unverified
    }
}
