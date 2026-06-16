use crate::disputes::dto::{DisputeResult, EscalateDisputeCommand};
use crate::errors::ApplicationError;
use domain::repositories::DisputeRepository;
use std::sync::Arc;
use tracing::instrument;

#[derive(Clone)]
pub struct EscalateDispute {
    dispute_repo: Arc<dyn DisputeRepository>,
}

impl EscalateDispute {
    pub fn new(dispute_repo: Arc<dyn DisputeRepository>) -> Self {
        Self { dispute_repo }
    }

    #[instrument(skip(self, cmd), fields(dispute_id = %cmd.dispute_id))]
    pub async fn execute(
        &self,
        cmd: EscalateDisputeCommand,
    ) -> Result<DisputeResult, ApplicationError> {
        // 1. Load dispute.
        let dispute = self
            .dispute_repo
            .find_by_id(cmd.dispute_id)
            .await?
            .ok_or(ApplicationError::DisputeNotFound)?;

        // 2. Apply domain escalation.
        let mut dispute = dispute;
        dispute.escalate()?;

        // 3. Persist.
        self.dispute_repo
            .escalate(cmd.dispute_id, cmd.actor_user_id, cmd.notes)
            .await?;

        // 4. Return refreshed dispute.
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
