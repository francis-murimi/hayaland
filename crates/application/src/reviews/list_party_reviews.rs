use crate::errors::ApplicationError;
use crate::reviews::dto::{ListPartyReviewsQuery, ReviewListResult};
use domain::repositories::ReviewSearchCriteria;
use domain::repositories::{PartyRepository, ReviewRepository};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ListPartyReviews {
    party_repo: Arc<dyn PartyRepository>,
    review_repo: Arc<dyn ReviewRepository>,
}

impl ListPartyReviews {
    pub fn new(
        party_repo: Arc<dyn PartyRepository>,
        review_repo: Arc<dyn ReviewRepository>,
    ) -> Self {
        Self {
            party_repo,
            review_repo,
        }
    }

    pub async fn execute(
        &self,
        party_id: Uuid,
        query: ListPartyReviewsQuery,
    ) -> Result<ReviewListResult, ApplicationError> {
        // Verify the target party exists.
        if self.party_repo.find_by_id(party_id).await?.is_none() {
            return Err(ApplicationError::PartyNotFound);
        }

        let viewing_own = query
            .actor_party_id
            .map(|id| id == party_id)
            .unwrap_or(false);

        let criteria = ReviewSearchCriteria {
            reviewed_party_id: Some(party_id),
            is_public: if query.is_admin || viewing_own {
                None
            } else {
                Some(true)
            },
            limit: query.limit.max(1),
            offset: query.offset.max(0),
            ..Default::default()
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
