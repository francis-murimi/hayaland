use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateDisputeRequest {
    #[serde(rename = "againstPartyId")]
    pub against_party_id: Option<Uuid>,
    #[serde(rename = "disputeType")]
    #[validate(length(min = 1))]
    pub dispute_type: String,
    #[validate(length(min = 1))]
    pub description: String,
    #[serde(rename = "evidenceUrls")]
    pub evidence_urls: Vec<String>,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct SubmitEvidenceRequest {
    #[serde(rename = "evidenceUrls")]
    pub evidence_urls: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct RespondToDisputeRequest {
    #[validate(length(min = 1))]
    pub message: String,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct EscalateDisputeRequest {
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct ResolveDisputeRequest {
    #[serde(rename = "resolutionType")]
    #[validate(length(min = 1))]
    pub resolution_type: String,
    #[serde(rename = "resolutionOutcome")]
    #[validate(length(min = 1))]
    pub resolution_outcome: String,
    #[validate(length(min = 1))]
    pub severity: String,
    #[serde(rename = "resolutionNotes")]
    pub resolution_notes: Option<String>,
    #[serde(rename = "nextDealStatus")]
    #[validate(length(min = 1))]
    pub next_deal_status: String,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct RejectDisputeRequest {
    #[validate(length(min = 1))]
    pub reason: String,
    #[serde(rename = "nextDealStatus")]
    pub next_deal_status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisputeResponseItem {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub party_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
    pub created_at: OffsetDateTime,
}

impl From<application::disputes::dto::DisputeResponseResult> for DisputeResponseItem {
    fn from(r: application::disputes::dto::DisputeResponseResult) -> Self {
        Self {
            id: r.id,
            dispute_id: r.dispute_id,
            party_id: r.party_id,
            user_id: r.user_id,
            message: r.message,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisputeResponse {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub raised_by_party_id: Uuid,
    pub raised_by_user_id: Uuid,
    pub against_party_id: Option<Uuid>,
    pub dispute_type: String,
    pub dispute_status: String,
    pub resolution_type: Option<String>,
    pub resolution_outcome: Option<String>,
    pub severity: Option<String>,
    pub description: String,
    pub evidence_urls: Vec<String>,
    pub admin_notes: Option<String>,
    pub resolution_notes: Option<String>,
    pub resolved_by_user_id: Option<Uuid>,
    pub resolved_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub responses: Vec<DisputeResponseItem>,
}

impl From<application::disputes::dto::DisputeResult> for DisputeResponse {
    fn from(r: application::disputes::dto::DisputeResult) -> Self {
        Self {
            id: r.id,
            deal_id: r.deal_id,
            raised_by_party_id: r.raised_by_party_id,
            raised_by_user_id: r.raised_by_user_id,
            against_party_id: r.against_party_id,
            dispute_type: r.dispute_type,
            dispute_status: r.dispute_status,
            resolution_type: r.resolution_type,
            resolution_outcome: r.resolution_outcome,
            severity: r.severity,
            description: r.description,
            evidence_urls: r.evidence_urls,
            admin_notes: r.admin_notes,
            resolution_notes: r.resolution_notes,
            resolved_by_user_id: r.resolved_by_user_id,
            resolved_at: r.resolved_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            responses: r.responses.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisputesResponse {
    pub disputes: Vec<DisputeResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

impl From<application::disputes::dto::DisputeListResult> for DisputesResponse {
    fn from(r: application::disputes::dto::DisputeListResult) -> Self {
        Self {
            disputes: r.disputes.into_iter().map(Into::into).collect(),
            total: r.total,
            limit: r.limit,
            offset: r.offset,
        }
    }
}
