use crate::errors::ApplicationError;
use crate::milestones::access::{allow_milestone_mutations, ensure_participant};
use crate::milestones::dto::{MilestoneActionCommand, MilestoneResult};
use domain::entities::MilestoneStatus;
use domain::repositories::{DealRepository, MilestoneRepository, PartyRepository};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Clone)]
pub struct StartMilestone {
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    milestone_repo: Arc<dyn MilestoneRepository>,
}

impl StartMilestone {
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
            cmd.is_admin,
        )
        .await?;

        let deal = self
            .deal_repo
            .find_by_id(milestone.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;
        allow_milestone_mutations(deal.deal_status)?;

        let mut milestone = milestone;
        if cmd.is_admin {
            milestone.milestone_status = MilestoneStatus::InProgress;
            milestone.completion_percentage = Decimal::from(25);
            milestone.updated_at = time::OffsetDateTime::now_utc();
        } else {
            milestone.start(cmd.actor_party_id)?;
        }
        self.milestone_repo.update(&milestone).await?;

        info!(milestone_id = %milestone.id, "started milestone");
        Ok(milestone.into())
    }
}
