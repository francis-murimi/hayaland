use crate::entities::Milestone;
use crate::errors::DomainError;
use async_trait::async_trait;
use rust_decimal::Decimal;
use uuid::Uuid;

#[async_trait]
pub trait MilestoneRepository: Send + Sync {
    async fn create(&self, milestone: &Milestone) -> Result<(), DomainError>;

    async fn update(&self, milestone: &Milestone) -> Result<(), DomainError>;

    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Milestone>, DomainError>;

    async fn find_by_deal(
        &self,
        deal_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Milestone>, DomainError>;

    async fn count_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError>;

    async fn count_verified_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError>;

    async fn count_by_status(&self, deal_id: Uuid, status: &str) -> Result<i64, DomainError>;
}

/// Summarised deal progress based on milestone data.
#[derive(Debug, Clone, PartialEq)]
pub struct DealProgress {
    pub deal_id: Uuid,
    pub total_milestones: i64,
    pub verified_milestones: i64,
    pub completed_milestones: i64,
    pub in_progress_milestones: i64,
    pub missed_milestones: i64,
    pub overall_completion_percentage: Decimal,
}
