use crate::entities::{MatchStatus, MatchSuggestion};
use crate::errors::DomainError;
use async_trait::async_trait;
use rust_decimal::Decimal;
use time::OffsetDateTime;
use uuid::Uuid;

/// Filters for listing match suggestions.
#[derive(Debug, Clone, Default)]
pub struct MatchFilters {
    pub status: Option<MatchStatus>,
    pub min_score: Option<f64>,
    pub max_score: Option<f64>,
    pub generated_by: Option<String>,
    pub created_after: Option<OffsetDateTime>,
    pub created_before: Option<OffsetDateTime>,
    pub limit: i64,
    pub offset: i64,
}

/// Counts of suggestions grouped by status for a party.
#[derive(Debug, Clone, Default)]
pub struct MatchCountByStatus {
    pub pending: i64,
    pub accepted: i64,
    pub declined: i64,
    pub counter_proposed: i64,
    pub expired: i64,
    pub converted_to_deal: i64,
}

#[async_trait]
pub trait MatchRepository: Send + Sync {
    /// Persist a new match suggestion.
    async fn create(&self, suggestion: &MatchSuggestion) -> Result<(), DomainError>;

    /// Find a suggestion by ID.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<MatchSuggestion>, DomainError>;

    /// List suggestions where the given party is a participant, optionally filtered by role.
    async fn list_for_party(
        &self,
        party_id: Uuid,
        role: Option<crate::entities::DealRole>,
        filters: &MatchFilters,
    ) -> Result<Vec<MatchSuggestion>, DomainError>;

    /// List all suggestions (admin use).
    async fn list_all(&self, filters: &MatchFilters) -> Result<Vec<MatchSuggestion>, DomainError>;

    /// Update the status of a suggestion.
    async fn update_status(
        &self,
        id: Uuid,
        status: MatchStatus,
        notes: Option<String>,
    ) -> Result<(), DomainError>;

    /// Update a counter-proposal.
    async fn update_counter_proposal(
        &self,
        id: Uuid,
        value: Option<Decimal>,
        notes: Option<String>,
    ) -> Result<(), DomainError>;

    /// Record that a suggestion was converted to a deal.
    async fn set_converted_deal(&self, id: Uuid, deal_id: Uuid) -> Result<(), DomainError>;

    /// Delete suggestions involving a specific party.
    async fn delete_by_party(
        &self,
        party_id: Uuid,
        status: Option<MatchStatus>,
    ) -> Result<u64, DomainError>;

    /// Delete all suggestions.
    async fn delete_all(&self) -> Result<u64, DomainError>;

    /// Count suggestions grouped by status for a party.
    async fn count_by_status(&self, party_id: Uuid) -> Result<MatchCountByStatus, DomainError>;

    /// Count suggestions grouped by status across the platform (admin use).
    async fn count_all_by_status(&self) -> Result<MatchCountByStatus, DomainError>;

    /// Check whether an equivalent pending suggestion already exists for the triplet.
    async fn find_existing_pending(
        &self,
        supplier_party_id: Uuid,
        consumer_party_id: Uuid,
        enhancer_party_id: Uuid,
    ) -> Result<Option<MatchSuggestion>, DomainError>;
}
