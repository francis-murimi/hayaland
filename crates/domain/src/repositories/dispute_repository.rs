use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::entities::{Dispute, DisputeResponse, DisputeStatus};
use crate::errors::DomainError;

/// Filters for the admin dispute list.
#[derive(Debug, Clone, Default)]
pub struct DisputeFilters {
    pub status: Option<DisputeStatus>,
    pub deal_id: Option<Uuid>,
    pub raised_by_party_id: Option<Uuid>,
    pub against_party_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

/// Result of a dispute list query.
#[derive(Debug, Clone)]
pub struct DisputeListResult {
    pub disputes: Vec<Dispute>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Outbound port for persisting and retrieving disputes and their responses.
#[async_trait]
pub trait DisputeRepository: Send + Sync {
    /// Persist a new dispute.
    async fn create(&self, dispute: &Dispute) -> Result<(), DomainError>;

    /// Fetch a dispute by ID.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Dispute>, DomainError>;

    /// List disputes for a deal with pagination.
    async fn list_by_deal(
        &self,
        deal_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<DisputeListResult, DomainError>;

    /// List disputes for the admin queue with filters.
    async fn list_admin(&self, filters: &DisputeFilters) -> Result<DisputeListResult, DomainError>;

    /// Append evidence URLs and optional admin notes. Optionally moves status to UNDER_REVIEW.
    async fn submit_evidence(
        &self,
        id: Uuid,
        evidence_urls: Vec<String>,
        notes: Option<String>,
    ) -> Result<(), DomainError>;

    /// Persist a response to a dispute.
    async fn add_response(&self, response: &DisputeResponse) -> Result<(), DomainError>;

    /// List responses for a dispute, ordered by created_at ASC.
    async fn list_responses(&self, dispute_id: Uuid) -> Result<Vec<DisputeResponse>, DomainError>;

    /// Escalate a dispute and record optional admin notes.
    async fn escalate(
        &self,
        id: Uuid,
        escalated_by_user_id: Uuid,
        notes: Option<String>,
    ) -> Result<(), DomainError>;

    /// Resolve a dispute. Fails if already terminal.
    async fn resolve(
        &self,
        id: Uuid,
        resolved_by_user_id: Uuid,
        resolution_type: crate::entities::ResolutionType,
        resolution_outcome: crate::entities::ResolutionOutcome,
        severity: crate::entities::DisputeSeverity,
        resolution_notes: Option<String>,
    ) -> Result<(), DomainError>;

    /// Reject a dispute. Fails if already terminal.
    async fn reject(
        &self,
        id: Uuid,
        resolved_by_user_id: Uuid,
        reason: String,
    ) -> Result<(), DomainError>;

    /// Count open disputes raised by a party.
    async fn count_open_by_party(&self, party_id: Uuid) -> Result<i64, DomainError>;

    /// Count open disputes raised against a party.
    async fn count_open_against_party(&self, party_id: Uuid) -> Result<i64, DomainError>;

    /// Update the `deals_disputed_count` column in trust_scores for a party.
    async fn increment_deals_disputed_count(&self, party_id: Uuid) -> Result<(), DomainError>;

    /// Update the dispute status and updated_at timestamp.
    async fn update_status(
        &self,
        id: Uuid,
        status: DisputeStatus,
        updated_at: OffsetDateTime,
    ) -> Result<(), DomainError>;
}
