use crate::errors::ApplicationError;
use crate::milestones::access::{allow_milestone_mutations, ensure_participant};
use crate::milestones::dto::{CreateMilestoneCommand, MilestoneResult};
use domain::entities::Milestone;
use domain::repositories::{DealRepository, MilestoneRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

#[derive(Clone)]
pub struct CreateMilestone {
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    milestone_repo: Arc<dyn MilestoneRepository>,
}

impl CreateMilestone {
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

    #[instrument(skip(self, cmd), fields(deal_id = %cmd.deal_id))]
    pub async fn execute(
        &self,
        cmd: CreateMilestoneCommand,
    ) -> Result<MilestoneResult, ApplicationError> {
        ensure_participant(
            &self.party_repo,
            &self.deal_repo,
            cmd.actor_user_id,
            cmd.actor_party_id,
            cmd.deal_id,
        )
        .await?;

        let deal = self
            .deal_repo
            .find_by_id(cmd.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;
        allow_milestone_mutations(deal.deal_status)?;

        self.ensure_party_participates(cmd.deal_id, cmd.assigned_to_party_id)
            .await?;
        self.ensure_party_participates(cmd.deal_id, cmd.verified_by_party_id)
            .await?;

        let milestone = Milestone::new(
            Uuid::now_v7(),
            cmd.deal_id,
            cmd.milestone_name,
            cmd.description,
            cmd.assigned_to_party_id,
            cmd.verified_by_party_id,
            cmd.due_date,
            cmd.completion_criteria,
            cmd.payment_trigger_amount,
            cmd.display_order,
        )?;

        self.milestone_repo.create(&milestone).await?;

        info!(milestone_id = %milestone.id, "created milestone");
        Ok(milestone.into())
    }

    async fn ensure_party_participates(
        &self,
        deal_id: Uuid,
        party_id: Uuid,
    ) -> Result<(), ApplicationError> {
        if !self
            .deal_repo
            .is_party_participant(deal_id, party_id)
            .await?
        {
            return Err(ApplicationError::Validation(vec![format!(
                "party {party_id} is not a participant in the deal"
            )]));
        }
        Ok(())
    }
}
