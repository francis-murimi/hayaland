use crate::errors::ApplicationError;
use crate::ports::TrustScoreRecalculationPort;
use crate::reviews::dto::{ReviewResult, SubmitReviewCommand};
use domain::entities::{DealStatus, Review, ReviewRating, ReviewText};
use domain::repositories::{DealRepository, PartyRepository, ReviewRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

#[derive(Clone)]
pub struct SubmitReview {
    review_repo: Arc<dyn ReviewRepository>,
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
    recalc: Arc<dyn TrustScoreRecalculationPort>,
}

impl SubmitReview {
    pub fn new(
        review_repo: Arc<dyn ReviewRepository>,
        deal_repo: Arc<dyn DealRepository>,
        party_repo: Arc<dyn PartyRepository>,
        recalc: Arc<dyn TrustScoreRecalculationPort>,
    ) -> Self {
        Self {
            review_repo,
            deal_repo,
            party_repo,
            recalc,
        }
    }

    #[instrument(skip(self, cmd), fields(deal_id = %cmd.deal_id))]
    pub async fn execute(
        &self,
        cmd: SubmitReviewCommand,
    ) -> Result<ReviewResult, ApplicationError> {
        // 1. Load deal aggregate.
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(cmd.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        let deal = aggregate.deal;
        let participations = aggregate.participations;

        // 2. Reviews can be submitted while executing (after milestones are verified)
        //    or after the deal is already completed.
        if deal.deal_status != DealStatus::Executing && deal.deal_status != DealStatus::Completed {
            return Err(ApplicationError::Validation(vec![
                "reviews can only be submitted while the deal is executing or completed"
                    .to_string(),
            ]));
        }

        // 3. Caller must be a member of the acting party (reviewer), unless admin.
        if !cmd.is_admin
            && !self
                .party_repo
                .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
                .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        // 4. Reviewer and reviewed must both be participants, and must be distinct.
        let reviewer = participations
            .iter()
            .find(|p| p.party_id == cmd.actor_party_id)
            .ok_or(ApplicationError::DealAccessDenied)?;

        let reviewed = participations
            .iter()
            .find(|p| p.party_id == cmd.reviewed_party_id)
            .ok_or(ApplicationError::DealAccessDenied)?;

        if reviewer.party_id == reviewed.party_id {
            return Err(ApplicationError::Validation(vec![
                "parties cannot review themselves".to_string(),
            ]));
        }

        // 5. No duplicate review.
        if self
            .review_repo
            .exists(cmd.deal_id, reviewer.party_id, reviewed.party_id)
            .await?
        {
            return Err(ApplicationError::DuplicateReview);
        }

        // 6. Build domain value objects.
        let overall = ReviewRating::new(cmd.overall_rating)?;
        let communication = cmd
            .communication_rating
            .map(ReviewRating::new)
            .transpose()?;
        let reliability = cmd.reliability_rating.map(ReviewRating::new).transpose()?;
        let quality = cmd.quality_rating.map(ReviewRating::new).transpose()?;
        let timeliness = cmd.timeliness_rating.map(ReviewRating::new).transpose()?;
        let text = cmd.review_text.map(|t| ReviewText::new(&t)).transpose()?;

        let mut review = Review::new(
            Uuid::now_v7(),
            cmd.deal_id,
            reviewer.party_id,
            reviewed.party_id,
            reviewed.role,
            overall,
            communication,
            reliability,
            quality,
            timeliness,
            text,
            cmd.is_public.unwrap_or(true),
        );

        // A review is "verified" when it is tied to a completed deal with accepted participations.
        review.is_verified = true;

        // 7. Persist.
        self.review_repo.create(&review).await?;

        info!(
            review_id = %review.id,
            reviewed_party_id = %review.reviewed_party_id,
            "review submitted"
        );

        // 8. Request trust-score recalculation for the reviewed party.
        self.recalc
            .request_recalculation(review.reviewed_party_id)
            .await?;

        Ok(review.into())
    }
}
