use crate::entities::trust_score::{
    DisputeInput, ResponseMetrics, ReviewInput, RoleDealInput, TrustScoreRow,
};
use crate::errors::DomainError;
use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

#[async_trait]
pub trait TrustScoreRepository: Send + Sync {
    async fn find_by_party_id(&self, party_id: Uuid) -> Result<Option<TrustScoreRow>, DomainError>;
    async fn create_default(&self, party_id: Uuid) -> Result<(), DomainError>;
    async fn upsert(&self, row: &TrustScoreRow) -> Result<(), DomainError>;
    async fn increment_deals_completed_count(
        &self,
        party_id: Uuid,
        deal_value: f64,
    ) -> Result<(), DomainError>;
    async fn increment_deals_cancelled_count(&self, party_id: Uuid) -> Result<(), DomainError>;
    async fn increment_deals_disputed_count(&self, party_id: Uuid) -> Result<(), DomainError>;
    async fn increment_timeouts_count(&self, party_id: Uuid) -> Result<(), DomainError>;
    async fn increment_no_shows_count(&self, party_id: Uuid) -> Result<(), DomainError>;
    async fn update_profile_completeness(
        &self,
        party_id: Uuid,
        completeness: f64,
    ) -> Result<(), DomainError>;
    async fn update_response_hours(
        &self,
        party_id: Uuid,
        hours: Option<f64>,
    ) -> Result<(), DomainError>;
    async fn update_verification_level(
        &self,
        party_id: Uuid,
        level: i32,
    ) -> Result<(), DomainError>;
    async fn update_public_cache(&self, party_id: Uuid, score: f64) -> Result<(), DomainError>;
    async fn list_party_ids(&self, limit: i64, offset: i64) -> Result<Vec<Uuid>, DomainError>;
    async fn find_review_inputs(&self, party_id: Uuid) -> Result<Vec<ReviewInput>, DomainError>;
    async fn find_dispute_inputs(&self, party_id: Uuid) -> Result<Vec<DisputeInput>, DomainError>;
    async fn find_role_deal_inputs(
        &self,
        party_id: Uuid,
    ) -> Result<HashMap<String, RoleDealInput>, DomainError>;
    async fn find_role_reviews(
        &self,
        party_id: Uuid,
    ) -> Result<HashMap<String, Vec<ReviewInput>>, DomainError>;
    async fn compute_response_metrics(
        &self,
        party_id: Uuid,
    ) -> Result<ResponseMetrics, DomainError>;
    async fn find_account_age_and_activity(
        &self,
        party_id: Uuid,
    ) -> Result<(i64, Option<i64>), DomainError>;
}
