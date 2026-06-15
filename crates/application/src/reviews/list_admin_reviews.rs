use crate::errors::ApplicationError;
use crate::reviews::dto::{AdminReviewListQuery, ReviewListResult};
use domain::repositories::ReviewRepository;
use domain::repositories::ReviewSearchCriteria;
use std::sync::Arc;

#[derive(Clone)]
pub struct ListAdminReviews {
    review_repo: Arc<dyn ReviewRepository>,
}

impl ListAdminReviews {
    pub fn new(review_repo: Arc<dyn ReviewRepository>) -> Self {
        Self { review_repo }
    }

    pub async fn execute(
        &self,
        query: AdminReviewListQuery,
    ) -> Result<ReviewListResult, ApplicationError> {
        let criteria = ReviewSearchCriteria {
            deal_id: query.deal_id,
            reviewer_party_id: query.reviewer_party_id,
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
