use crate::errors::ApplicationError;
use domain::repositories::ReviewRepository;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct HideReview {
    review_repo: Arc<dyn ReviewRepository>,
}

impl HideReview {
    pub fn new(review_repo: Arc<dyn ReviewRepository>) -> Self {
        Self { review_repo }
    }

    pub async fn execute(
        &self,
        review_id: Uuid,
        platform_response: Option<String>,
    ) -> Result<(), ApplicationError> {
        // hide() sets is_public = false, clears review_text, and records the admin response.
        self.review_repo.hide(review_id, platform_response).await?;
        Ok(())
    }
}
