use crate::errors::ApplicationError;
use crate::milestones::access::ensure_participant;
use crate::milestones::dto::{DealProgressResult, GetDealProgressQuery};
use domain::entities::Currency;
use domain::repositories::{DealRepository, MilestoneRepository, PartyRepository};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Clone)]
pub struct GetDealProgress {
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    milestone_repo: Arc<dyn MilestoneRepository>,
}

impl GetDealProgress {
    pub fn new(
        party_repo: Arc<dyn PartyRepository>,
        deal_repo: Arc<dyn DealRepository>,
        milestone_repo: Arc<dyn MilestoneRepository>,
    ) -> Self {
        Self {
            party_repo,
            deal_repo,
            milestone_repo,
        }
    }

    #[instrument(skip(self, query), fields(deal_id = %query.deal_id))]
    pub async fn execute(
        &self,
        query: GetDealProgressQuery,
    ) -> Result<DealProgressResult, ApplicationError> {
        ensure_participant(
            &self.party_repo,
            &self.deal_repo,
            query.actor_user_id,
            query.actor_party_id,
            query.deal_id,
        )
        .await?;

        let total = self.milestone_repo.count_by_deal(query.deal_id).await?;
        let verified = self
            .milestone_repo
            .count_by_status(query.deal_id, "VERIFIED")
            .await?;
        let completed = self
            .milestone_repo
            .count_by_status(query.deal_id, "COMPLETED")
            .await?;
        let in_progress = self
            .milestone_repo
            .count_by_status(query.deal_id, "IN_PROGRESS")
            .await?;
        let missed = self
            .milestone_repo
            .count_by_status(query.deal_id, "MISSED")
            .await?;

        let overall = if total == 0 {
            Decimal::ZERO
        } else if verified == total {
            Decimal::from(100)
        } else {
            let milestones = self
                .milestone_repo
                .find_by_deal(query.deal_id, total, 0)
                .await?;
            let sum: Decimal = milestones.iter().map(|m| m.completion_percentage).sum();
            (sum / Decimal::from(total)).round_dp(2)
        };

        info!(deal_id = %query.deal_id, verified, total, "fetched deal progress");

        Ok(DealProgressResult {
            deal_id: query.deal_id,
            total_milestones: total,
            verified_milestones: verified,
            completed_milestones: completed,
            in_progress_milestones: in_progress,
            missed_milestones: missed,
            overall_completion_percentage: overall,
            currency: Currency::Points,
        })
    }
}
