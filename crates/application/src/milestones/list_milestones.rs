use crate::errors::ApplicationError;
use crate::milestones::access::ensure_participant;
use crate::milestones::dto::{ListMilestonesQuery, ListMilestonesResult, MilestoneResult};
use domain::repositories::{DealRepository, MilestoneRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Clone)]
pub struct ListMilestones {
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    milestone_repo: Arc<dyn MilestoneRepository>,
}

impl ListMilestones {
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
        query: ListMilestonesQuery,
    ) -> Result<ListMilestonesResult, ApplicationError> {
        ensure_participant(
            &self.party_repo,
            &self.deal_repo,
            query.actor_user_id,
            query.actor_party_id,
            query.deal_id,
            query.is_admin,
        )
        .await?;

        let limit = query.limit.unwrap_or(50).clamp(1, 100);
        let offset = query.offset.unwrap_or(0).max(0);

        let milestones = self
            .milestone_repo
            .find_by_deal(query.deal_id, limit, offset)
            .await?;
        let total = self.milestone_repo.count_by_deal(query.deal_id).await?;

        info!(deal_id = %query.deal_id, total, "listed milestones");

        Ok(ListMilestonesResult {
            milestones: milestones.into_iter().map(MilestoneResult::from).collect(),
            total,
            limit,
            offset,
        })
    }
}
