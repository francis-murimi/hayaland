use crate::disputes::dto::{DisputeResult, RejectDisputeCommand};
use crate::errors::ApplicationError;
use domain::entities::DealStatus;
use domain::repositories::{DealRepository, DisputeRepository};
use std::sync::Arc;
use tracing::instrument;

#[derive(Clone)]
pub struct RejectDispute {
    dispute_repo: Arc<dyn DisputeRepository>,
    deal_repo: Arc<dyn DealRepository>,
}

impl RejectDispute {
    pub fn new(
        dispute_repo: Arc<dyn DisputeRepository>,
        deal_repo: Arc<dyn DealRepository>,
    ) -> Self {
        Self {
            dispute_repo,
            deal_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(dispute_id = %cmd.dispute_id))]
    pub async fn execute(
        &self,
        cmd: RejectDisputeCommand,
    ) -> Result<DisputeResult, ApplicationError> {
        // 1. Load dispute.
        let dispute = self
            .dispute_repo
            .find_by_id(cmd.dispute_id)
            .await?
            .ok_or(ApplicationError::DisputeNotFound)?;

        // 2. Load deal.
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(dispute.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;
        let mut deal = aggregate.deal;

        // 3. Apply domain rejection.
        let mut dispute = dispute;
        dispute.reject(cmd.reason.clone(), cmd.actor_user_id)?;

        // 4. Persist rejection.
        self.dispute_repo
            .reject(cmd.dispute_id, cmd.actor_user_id, cmd.reason)
            .await?;

        // 5. Optionally transition deal.
        if let Some(next_status_str) = cmd.next_deal_status {
            let next_status =
                DealStatus::try_from(next_status_str.as_str()).map_err(ApplicationError::from)?;
            if !matches!(
                next_status,
                DealStatus::Executing | DealStatus::Completed | DealStatus::Cancelled
            ) {
                return Err(ApplicationError::InvalidStateTransition {
                    from: "DISPUTED".to_string(),
                    to: next_status.as_str().to_string(),
                });
            }
            deal.transition(next_status)?;
            self.deal_repo.update(&deal).await?;
        }

        self.deal_repo
            .record_history(
                dispute.deal_id,
                "DISPUTE_REJECTED",
                None,
                Some(serde_json::json!({"dispute_id": dispute.id})),
            )
            .await?;

        // 6. Return refreshed dispute.
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
