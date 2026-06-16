use domain::entities::{Dispute, DisputeResponse};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Command to raise a new dispute.
#[derive(Debug, Clone, Deserialize)]
pub struct RaiseDisputeCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub deal_id: Uuid,
    pub against_party_id: Option<Uuid>,
    pub dispute_type: String,
    pub description: String,
    pub evidence_urls: Vec<String>,
}

/// Command to submit additional evidence.
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitEvidenceCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub dispute_id: Uuid,
    pub evidence_urls: Vec<String>,
    pub notes: Option<String>,
}

/// Command to post a response to a dispute.
#[derive(Debug, Clone, Deserialize)]
pub struct RespondToDisputeCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub dispute_id: Uuid,
    pub message: String,
}

/// Command to escalate a dispute (admin only).
#[derive(Debug, Clone, Deserialize)]
pub struct EscalateDisputeCommand {
    pub actor_user_id: Uuid,
    pub dispute_id: Uuid,
    pub notes: Option<String>,
}

/// Command to resolve a dispute (admin only).
#[derive(Debug, Clone, Deserialize)]
pub struct ResolveDisputeCommand {
    pub actor_user_id: Uuid,
    pub dispute_id: Uuid,
    pub resolution_type: String,
    pub resolution_outcome: String,
    pub severity: String,
    pub resolution_notes: Option<String>,
    pub next_deal_status: String,
}

/// Command to reject a dispute (admin only).
#[derive(Debug, Clone, Deserialize)]
pub struct RejectDisputeCommand {
    pub actor_user_id: Uuid,
    pub dispute_id: Uuid,
    pub reason: String,
    pub next_deal_status: Option<String>,
}

/// Query to list disputes for a deal.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ListDealDisputesQuery {
    pub deal_id: Uuid,
    pub limit: i64,
    pub offset: i64,
}

impl Default for ListDealDisputesQuery {
    fn default() -> Self {
        Self {
            deal_id: Uuid::nil(),
            limit: 20,
            offset: 0,
        }
    }
}

/// Query to fetch a single dispute.
#[derive(Debug, Clone, Deserialize)]
pub struct GetDisputeQuery {
    pub dispute_id: Uuid,
}

/// Query for the admin dispute queue.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AdminDisputeListQuery {
    pub status: Option<String>,
    pub deal_id: Option<Uuid>,
    pub raised_by_party_id: Option<Uuid>,
    pub against_party_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for AdminDisputeListQuery {
    fn default() -> Self {
        Self {
            status: None,
            deal_id: None,
            raised_by_party_id: None,
            against_party_id: None,
            limit: 20,
            offset: 0,
        }
    }
}

/// A dispute response as returned by application use cases.
#[derive(Debug, Clone, Serialize)]
pub struct DisputeResponseResult {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub party_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
    pub created_at: OffsetDateTime,
}

impl From<DisputeResponse> for DisputeResponseResult {
    fn from(r: DisputeResponse) -> Self {
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

/// A dispute as returned by application use cases.
#[derive(Debug, Clone, Serialize)]
pub struct DisputeResult {
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
    pub responses: Vec<DisputeResponseResult>,
}

impl From<Dispute> for DisputeResult {
    fn from(d: Dispute) -> Self {
        Self {
            id: d.id,
            deal_id: d.deal_id,
            raised_by_party_id: d.raised_by_party_id,
            raised_by_user_id: d.raised_by_user_id,
            against_party_id: d.against_party_id,
            dispute_type: d.dispute_type.as_str().to_string(),
            dispute_status: d.dispute_status.as_str().to_string(),
            resolution_type: d.resolution_type.map(|r| r.as_str().to_string()),
            resolution_outcome: d.resolution_outcome.map(|r| r.as_str().to_string()),
            severity: d.severity.map(|s| s.as_str().to_string()),
            description: d.description,
            evidence_urls: d.evidence_urls,
            admin_notes: d.admin_notes,
            resolution_notes: d.resolution_notes,
            resolved_by_user_id: d.resolved_by_user_id,
            resolved_at: d.resolved_at,
            created_at: d.created_at,
            updated_at: d.updated_at,
            responses: Vec::new(),
        }
    }
}

/// Paginated list of disputes.
#[derive(Debug, Clone, Serialize)]
pub struct DisputeListResult {
    pub disputes: Vec<DisputeResult>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
