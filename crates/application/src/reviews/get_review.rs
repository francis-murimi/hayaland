use crate::errors::ApplicationError;
use crate::reviews::dto::{GetReviewQuery, ReviewResult};
use domain::repositories::{DealRepository, ReviewRepository};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct GetReview {
    deal_repo: Arc<dyn DealRepository>,
    review_repo: Arc<dyn ReviewRepository>,
}

impl GetReview {
    pub fn new(deal_repo: Arc<dyn DealRepository>, review_repo: Arc<dyn ReviewRepository>) -> Self {
        Self {
            deal_repo,
            review_repo,
        }
    }

    pub async fn execute(
        &self,
        review_id: Uuid,
        query: GetReviewQuery,
    ) -> Result<ReviewResult, ApplicationError> {
        let review = self
            .review_repo
            .find_by_id(review_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        // Private reviews are visible only to the reviewer, the reviewed party, and admins.
        if !review.is_public {
            let allowed = query.is_admin
                || query.actor_party_id == Some(review.reviewer_party_id)
                || query.actor_party_id == Some(review.reviewed_party_id);
            if !allowed {
                return Err(ApplicationError::DealAccessDenied);
            }
        }

        // Everyone else must at least be a deal participant or admin to see a public review.
        if !query.is_admin {
            let party_id = query.actor_party_id.ok_or(ApplicationError::Forbidden)?;
            if !self
                .deal_repo
                .is_party_participant(review.deal_id, party_id)
                .await?
            {
                return Err(ApplicationError::DealAccessDenied);
            }
        }

        Ok(review.into())
    }
}
