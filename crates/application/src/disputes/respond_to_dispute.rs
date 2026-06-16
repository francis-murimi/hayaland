use crate::disputes::dto::{DisputeResult, RespondToDisputeCommand};
use crate::errors::ApplicationError;
use domain::entities::DisputeResponse;
use domain::repositories::{DealRepository, DisputeRepository};
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

#[derive(Clone)]
pub struct RespondToDispute {
    deal_repo: Arc<dyn DealRepository>,
    dispute_repo: Arc<dyn DisputeRepository>,
}

impl RespondToDispute {
    pub fn new(
        deal_repo: Arc<dyn DealRepository>,
        dispute_repo: Arc<dyn DisputeRepository>,
    ) -> Self {
        Self {
            deal_repo,
            dispute_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(dispute_id = %cmd.dispute_id))]
    pub async fn execute(
        &self,
        cmd: RespondToDisputeCommand,
    ) -> Result<DisputeResult, ApplicationError> {
        // 1. Load dispute.
        let dispute = self
            .dispute_repo
            .find_by_id(cmd.dispute_id)
            .await?
            .ok_or(ApplicationError::DisputeNotFound)?;

        // 2. Load deal aggregate.
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(dispute.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        // 3. Actor must participate in the deal, unless admin.
        if !cmd.is_admin
            && !aggregate
                .participations
                .iter()
                .any(|p| p.party_id == cmd.actor_party_id)
        {
            return Err(ApplicationError::DealAccessDenied);
        }

        // 4. Create and persist response.
        let response = DisputeResponse::new(
            Uuid::now_v7(),
            cmd.dispute_id,
            cmd.actor_party_id,
            cmd.actor_user_id,
            cmd.message,
        );
        self.dispute_repo.add_response(&response).await?;

        // 5. Return refreshed dispute.
        let dispute = self
            .dispute_repo
            .find_by_id(cmd.dispute_id)
            .await?
            .ok_or(ApplicationError::DisputeNotFound)?;
        let responses = self.dispute_repo.list_responses(cmd.dispute_id).await?;

        let mut result: DisputeResult = dispute.into();
        result.responses = responses.into_iter().map(Into::into).collect();
        Ok(result)
    }
}
