use crate::errors::ApplicationError;
use crate::milestones::access::{allow_milestone_mutations, ensure_participant};
use crate::milestones::dto::{MilestoneResult, UpdateMilestoneCommand};
use domain::repositories::{DealRepository, MilestoneRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

#[derive(Clone)]
pub struct UpdateMilestone {
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    milestone_repo: Arc<dyn MilestoneRepository>,
}

impl UpdateMilestone {
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
        cmd: UpdateMilestoneCommand,
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

        if let Some(name) = cmd.milestone_name {
            milestone.set_name(name)?;
        }
        if let Some(description) = cmd.description {
            milestone.description = Some(description);
            milestone.updated_at = time::OffsetDateTime::now_utc();
        }
        if let Some(due_date) = cmd.due_date {
            milestone.due_date = Some(due_date);
            milestone.updated_at = time::OffsetDateTime::now_utc();
        }
        if let Some(criteria) = cmd.completion_criteria {
            milestone.set_completion_criteria(criteria)?;
        }
        if let Some(amount) = cmd.payment_trigger_amount {
            milestone.set_payment_trigger_amount(Some(amount))?;
        }
        if let Some(order) = cmd.display_order {
            milestone.display_order = order;
            milestone.updated_at = time::OffsetDateTime::now_utc();
        }
        if let Some(assigned) = cmd.assigned_to_party_id {
            self.ensure_party_participates(milestone.deal_id, assigned)
                .await?;
            milestone.assigned_to_party_id = assigned;
            milestone.updated_at = time::OffsetDateTime::now_utc();
        }
        if let Some(verifier) = cmd.verified_by_party_id {
            self.ensure_party_participates(milestone.deal_id, verifier)
                .await?;
            milestone.verified_by_party_id = verifier;
            milestone.updated_at = time::OffsetDateTime::now_utc();
        }

        self.milestone_repo.update(&milestone).await?;

        info!(milestone_id = %milestone.id, "updated milestone");
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
