use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::entities::{PartyVerification, PartyVerificationType};
use crate::errors::DomainError;

/// Criteria for filtering the admin verification queue.
#[derive(Debug, Clone, Default)]
pub struct VerificationListFilters {
    pub status: Option<String>,
    pub verification_type: Option<String>,
    pub party_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

/// Paginated result returned by admin verification listing.
#[derive(Debug, Clone)]
pub struct VerificationListResult {
    pub verifications: Vec<PartyVerification>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Repository port for party verification records.
#[async_trait]
pub trait PartyVerificationRepository: Send + Sync {
    /// Persist a new verification request.
    async fn create(&self, verification: &PartyVerification) -> Result<(), DomainError>;

    /// Find a verification by id.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PartyVerification>, DomainError>;

    /// Find an active (pending or approved) verification for a party and type.
    async fn find_active_by_party_and_type(
        &self,
        party_id: Uuid,
        verification_type: PartyVerificationType,
    ) -> Result<Option<PartyVerification>, DomainError>;

    /// List all verification records for a party.
    async fn list_by_party(&self, party_id: Uuid) -> Result<Vec<PartyVerification>, DomainError>;

    /// List verifications for the admin queue.
    async fn list(
        &self,
        filters: &VerificationListFilters,
    ) -> Result<VerificationListResult, DomainError>;

    /// Count verifications matching the admin queue filters.
    async fn count(&self, filters: &VerificationListFilters) -> Result<i64, DomainError>;

    /// Approve a pending verification.
    async fn approve(
        &self,
        id: Uuid,
        reviewed_by_user_id: Uuid,
        review_notes: Option<String>,
    ) -> Result<(), DomainError>;

    /// Reject a pending verification.
    async fn reject(
        &self,
        id: Uuid,
        reviewed_by_user_id: Uuid,
        rejection_reason: String,
        review_notes: Option<String>,
    ) -> Result<(), DomainError>;

    /// Revoke an approved verification.
    async fn revoke(
        &self,
        id: Uuid,
        reviewed_by_user_id: Uuid,
        reason: String,
        review_notes: Option<String>,
    ) -> Result<(), DomainError>;

    /// Sum points for all approved, non-expired verifications of a party.
    async fn sum_approved_points(&self, party_id: Uuid) -> Result<i64, DomainError>;

    /// Count verifications for a party grouped by status.
    async fn count_by_status(&self, party_id: Uuid, status: &str) -> Result<i64, DomainError>;

    /// Update the provider reference and payload for a verification (used by future automated providers).
    async fn set_provider_reference(
        &self,
        id: Uuid,
        provider_reference: String,
        provider_payload: Option<Value>,
    ) -> Result<(), DomainError>;

    /// Mark a verification as expired.
    async fn mark_expired(&self, id: Uuid) -> Result<(), DomainError>;

    /// Upsert the derived verification level for a party into the trust_scores table.
    async fn update_verification_level(
        &self,
        party_id: Uuid,
        verification_level: i32,
    ) -> Result<(), DomainError>;
}
