use async_trait::async_trait;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::entities::{Deal, DealParticipation, DealRole, DealStatus};
use crate::errors::DomainError;

/// Input for creating or updating a deal aggregate.
#[derive(Debug, Clone)]
pub struct DealAggregate {
    pub deal: Deal,
    pub participations: Vec<DealParticipation>,
}

/// Criteria for listing deals visible to a user.
#[derive(Debug, Clone, Default)]
pub struct DealSearchCriteria {
    pub party_id: Option<Uuid>,
    pub status: Option<DealStatus>,
    pub initiator_party_id: Option<Uuid>,
    pub domain_category_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

/// Result of a deal search/list.
#[derive(Debug, Clone)]
pub struct DealListResult {
    pub deals: Vec<Deal>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[async_trait]
pub trait DealRepository: Send + Sync {
    /// Save a new deal and its participations.
    async fn create(&self, aggregate: &DealAggregate) -> Result<(), DomainError>;

    /// Fetch a deal by ID (without participations).
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Deal>, DomainError>;

    /// Fetch a deal aggregate by ID.
    async fn find_aggregate_by_id(&self, id: Uuid) -> Result<Option<DealAggregate>, DomainError>;

    /// Fetch the full aggregate including participations, filtering by deal ID.
    async fn find_participations_by_deal(
        &self,
        deal_id: Uuid,
    ) -> Result<Vec<DealParticipation>, DomainError>;

    /// Update the deal (status, value, etc.).
    async fn update(&self, deal: &Deal) -> Result<(), DomainError>;

    /// Update a participation status.
    async fn update_participation(
        &self,
        participation: &DealParticipation,
    ) -> Result<(), DomainError>;

    /// List deals matching criteria. Visibility filtering is applied by the caller/application layer.
    async fn list(&self, criteria: &DealSearchCriteria) -> Result<DealListResult, DomainError>;

    /// Count active deals for a party (all non-terminal statuses).
    async fn count_active_deals_for_party(&self, party_id: Uuid) -> Result<i64, DomainError>;

    /// Count active deals for a party in a specific role.
    async fn count_active_deals_for_party_role(
        &self,
        party_id: Uuid,
        role: DealRole,
    ) -> Result<i64, DomainError>;

    /// Record a deal history event.
    async fn record_history(
        &self,
        deal_id: Uuid,
        event_type: &str,
        actor_party_id: Option<Uuid>,
        details: Option<serde_json::Value>,
    ) -> Result<(), DomainError>;

    /// Check whether a party is a participant in a deal.
    async fn is_party_participant(
        &self,
        deal_id: Uuid,
        party_id: Uuid,
    ) -> Result<bool, DomainError>;

    /// Generate the next human-readable deal reference.
    async fn next_deal_reference(&self) -> Result<String, DomainError>;

    /// Update the aggregate value distribution totals (platform fee, total value).
    async fn update_value_totals(
        &self,
        deal_id: Uuid,
        total_value: Decimal,
        platform_fee_percentage: Decimal,
        platform_fee_amount: Decimal,
    ) -> Result<(), DomainError>;
}
