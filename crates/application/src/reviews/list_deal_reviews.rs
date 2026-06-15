use crate::errors::ApplicationError;
use crate::reviews::dto::{ListDealReviewsQuery, ReviewListResult};
use domain::repositories::ReviewSearchCriteria;
use domain::repositories::{DealRepository, ReviewRepository};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ListDealReviews {
    deal_repo: Arc<dyn DealRepository>,
    review_repo: Arc<dyn ReviewRepository>,
}

impl ListDealReviews {
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
        query: ListDealReviewsQuery,
    ) -> Result<ReviewListResult, ApplicationError> {
        // Visibility: the caller must be a participant or an admin.
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

        let criteria = ReviewSearchCriteria {
            deal_id: Some(deal_id),
            reviewer_party_id: None,
            reviewed_party_id: query.reviewed_party_id,
            is_public: query.is_public,
            limit: query.limit.max(1),
            offset: query.offset.max(0),
        };

        let result = self.review_repo.list(&criteria).await?;
        Ok(ReviewListResult {
            reviews: result.reviews.into_iter().map(Into::into).collect(),
            total: result.total,
            limit: result.limit,
            offset: result.offset,
        })
    }
}
