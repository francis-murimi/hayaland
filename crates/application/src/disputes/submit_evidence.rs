use crate::disputes::dto::{DisputeResult, SubmitEvidenceCommand};
use crate::errors::ApplicationError;
use domain::repositories::{DealRepository, DisputeRepository};
use std::sync::Arc;
use tracing::instrument;

#[derive(Clone)]
pub struct SubmitEvidence {
    deal_repo: Arc<dyn DealRepository>,
    dispute_repo: Arc<dyn DisputeRepository>,
}

impl SubmitEvidence {
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
        cmd: SubmitEvidenceCommand,
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

        // 3. Only the raising party or an admin may submit evidence.
        if !cmd.is_admin && dispute.raised_by_party_id != cmd.actor_party_id {
            return Err(ApplicationError::DisputeAccessDenied);
        }

        // 4. Actor must participate in the deal (admin bypasses membership check only).
        if !cmd.is_admin
            && !aggregate
                .participations
                .iter()
                .any(|p| p.party_id == cmd.actor_party_id)
        {
            return Err(ApplicationError::DealAccessDenied);
        }

        // 5. Persist evidence. Repository also moves OPEN -> UNDER_REVIEW.
        self.dispute_repo
            .submit_evidence(cmd.dispute_id, cmd.evidence_urls, cmd.notes)
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
