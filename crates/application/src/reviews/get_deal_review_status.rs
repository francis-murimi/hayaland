use crate::errors::ApplicationError;
use crate::reviews::dto::DealReviewStatusResult;
use domain::repositories::{DealRepository, ReviewRepository};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct GetDealReviewStatus {
    deal_repo: Arc<dyn DealRepository>,
    review_repo: Arc<dyn ReviewRepository>,
}

impl GetDealReviewStatus {
    pub fn new(deal_repo: Arc<dyn DealRepository>, review_repo: Arc<dyn ReviewRepository>) -> Self {
        Self {
            deal_repo,
            review_repo,
        }
    }

    pub async fn execute(
        &self,
        deal_id: Uuid,
        _actor_user_id: Uuid,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<DealReviewStatusResult, ApplicationError> {
        // Visibility: only participants or admins may see review status.
        let visible = if is_admin {
            true
        } else if let Some(party_id) = actor_party_id {
            self.deal_repo
                .is_party_participant(deal_id, party_id)
                .await?
        } else {
            false
        };

        if !visible {
            return Err(ApplicationError::DealAccessDenied);
        }

        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        let pairs: Vec<(Uuid, _)> = aggregate
            .participations
            .iter()
            .map(|p| (p.party_id, p.role))
            .collect();

        let missing = self
            .review_repo
            .find_missing_review_pairs(deal_id, &pairs)
            .await?;

        let total_required = pairs.len().saturating_sub(1) * pairs.len();
        let total_received = total_required - missing.len();

        Ok(DealReviewStatusResult {
            deal_id,
            total_required: total_required as i64,
            total_received: total_received as i64,
            is_complete: missing.is_empty(),
            missing_pairs: missing,
        })
    }
}
