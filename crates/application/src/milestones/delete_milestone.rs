use crate::errors::ApplicationError;
use crate::milestones::access::{allow_milestone_mutations, ensure_participant};
use crate::milestones::dto::MilestoneActionCommand;
use domain::entities::MilestoneStatus;
use domain::repositories::{DealRepository, MilestoneRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Clone)]
pub struct DeleteMilestone {
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    milestone_repo: Arc<dyn MilestoneRepository>,
}

impl DeleteMilestone {
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
    pub async fn execute(&self, cmd: MilestoneActionCommand) -> Result<(), ApplicationError> {
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

        if milestone.milestone_status == MilestoneStatus::Verified {
            return Err(ApplicationError::Validation(vec![
                "verified milestones cannot be deleted".to_string(),
            ]));
        }

        self.milestone_repo.delete(cmd.milestone_id).await?;

        info!(milestone_id = %cmd.milestone_id, "deleted milestone");
        Ok(())
    }
}
