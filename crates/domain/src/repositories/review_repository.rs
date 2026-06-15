use async_trait::async_trait;
use uuid::Uuid;

use crate::entities::{DealRole, Review};
use crate::errors::DomainError;

#[derive(Debug, Clone, Default)]
pub struct ReviewSearchCriteria {
    pub deal_id: Option<Uuid>,
    pub reviewer_party_id: Option<Uuid>,
    pub reviewed_party_id: Option<Uuid>,
    pub is_public: Option<bool>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct ReviewListResult {
    pub reviews: Vec<Review>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[async_trait]
pub trait ReviewRepository: Send + Sync {
    /// Persist a new review.
    async fn create(&self, review: &Review) -> Result<(), DomainError>;

    /// Find a review by id.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Review>, DomainError>;

    /// Check whether a review already exists for the (deal, reviewer, reviewed) tuple.
    async fn exists(
        &self,
        deal_id: Uuid,
        reviewer_party_id: Uuid,
        reviewed_party_id: Uuid,
    ) -> Result<bool, DomainError>;

    /// Count how many reviews exist for a deal.
    async fn count_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError>;

    /// Return the (reviewer_party_id, reviewed_party_id) pairs that do NOT have a review yet.
    async fn find_missing_review_pairs(
        &self,
        deal_id: Uuid,
        participations: &[(Uuid, DealRole)],
    ) -> Result<Vec<(Uuid, Uuid)>, DomainError>;

    /// List reviews matching the criteria.
    async fn list(&self, criteria: &ReviewSearchCriteria) -> Result<ReviewListResult, DomainError>;

    /// Count reviews matching the criteria.
    async fn count(&self, criteria: &ReviewSearchCriteria) -> Result<i64, DomainError>;

    /// Update an existing review (text / ratings / visibility). MVP optional.
    async fn update(&self, review: &Review) -> Result<(), DomainError>;

    /// Soft-delete a review by clearing public visibility and text. MVP optional.
    async fn hide(&self, id: Uuid, platform_response: Option<String>) -> Result<(), DomainError>;
}
