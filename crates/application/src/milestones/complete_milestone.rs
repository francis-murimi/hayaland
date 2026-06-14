use crate::errors::ApplicationError;
use crate::milestones::access::{allow_milestone_mutations, ensure_participant};
use crate::milestones::dto::{MilestoneActionCommand, MilestoneResult};
use domain::repositories::{DealRepository, MilestoneRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Clone)]
pub struct CompleteMilestone {
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    milestone_repo: Arc<dyn MilestoneRepository>,
}

impl CompleteMilestone {
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

    #[instrument(skip(self, cmd), fields(milestone_id = %cmd.milestone_id))]
    pub async fn execute(
        &self,
        cmd: MilestoneActionCommand,
    ) -> Result<MilestoneResult, ApplicationError> {
        let milestone = self
            .milestone_repo
            .find_by_id(cmd.milestone_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        ensure_participant(
            &self.party_repo,
            &self.deal_repo,
            cmd.actor_user_id,
            cmd.actor_party_id,
            milestone.deal_id,
        )
        .await?;

        let deal = self
            .deal_repo
            .find_by_id(milestone.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;
        allow_milestone_mutations(deal.deal_status)?;

        let mut milestone = milestone;
        milestone.complete(cmd.actor_party_id)?;
        self.milestone_repo.update(&milestone).await?;

        info!(milestone_id = %milestone.id, "completed milestone");
        Ok(milestone.into())
    }
}
