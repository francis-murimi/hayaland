use crate::disputes::dto::{DisputeResult, ResolveDisputeCommand};
use crate::errors::ApplicationError;
use crate::ports::TrustScoreRecalculationPort;
use domain::entities::{DealStatus, DisputeSeverity, ResolutionOutcome, ResolutionType};
use domain::repositories::{DealRepository, DisputeRepository};
use std::sync::Arc;
use tracing::instrument;

#[derive(Clone)]
pub struct ResolveDispute {
    dispute_repo: Arc<dyn DisputeRepository>,
    deal_repo: Arc<dyn DealRepository>,
    recalc: Arc<dyn TrustScoreRecalculationPort>,
}

impl ResolveDispute {
    pub fn new(
        dispute_repo: Arc<dyn DisputeRepository>,
        deal_repo: Arc<dyn DealRepository>,
        recalc: Arc<dyn TrustScoreRecalculationPort>,
    ) -> Self {
        Self {
            dispute_repo,
            deal_repo,
            recalc,
        }
    }

    #[instrument(skip(self, cmd), fields(dispute_id = %cmd.dispute_id))]
    pub async fn execute(
        &self,
        cmd: ResolveDisputeCommand,
    ) -> Result<DisputeResult, ApplicationError> {
        // 1. Parse enums.
        let resolution_type = ResolutionType::try_from(cmd.resolution_type.as_str())
            .map_err(ApplicationError::from)?;
        let resolution_outcome = ResolutionOutcome::try_from(cmd.resolution_outcome.as_str())
            .map_err(ApplicationError::from)?;
        let severity =
            DisputeSeverity::try_from(cmd.severity.as_str()).map_err(ApplicationError::from)?;
        let next_deal_status =
            DealStatus::try_from(cmd.next_deal_status.as_str()).map_err(ApplicationError::from)?;

        if !matches!(
            next_deal_status,
            DealStatus::Executing | DealStatus::Completed | DealStatus::Cancelled
        ) {
            return Err(ApplicationError::InvalidStateTransition {
                from: "DISPUTED".to_string(),
                to: next_deal_status.as_str().to_string(),
            });
        }

        // 2. Load dispute.
        let dispute = self
            .dispute_repo
            .find_by_id(cmd.dispute_id)
            .await?
            .ok_or(ApplicationError::DisputeNotFound)?;

        // 3. Load deal.
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(dispute.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;
        let mut deal = aggregate.deal;

        // 4. Apply domain resolution.
        let mut dispute = dispute;
        dispute.resolve(
            resolution_type,
            resolution_outcome,
            severity,
            cmd.resolution_notes.clone(),
            cmd.actor_user_id,
        )?;

        // 5. Transition deal.
        deal.transition(next_deal_status)?;

        // 6. Persist resolution and deal.
        self.dispute_repo
            .resolve(
                cmd.dispute_id,
                cmd.actor_user_id,
                resolution_type,
                resolution_outcome,
                severity,
                cmd.resolution_notes,
            )
            .await?;
        self.deal_repo.update(&deal).await?;
        self.deal_repo
            .record_history(
                dispute.deal_id,
                "DISPUTE_RESOLVED",
                None,
                Some(serde_json::json!({
                    "dispute_id": dispute.id,
                    "resolution_type": resolution_type.as_str(),
                    "resolution_outcome": resolution_outcome.as_str(),
                    "severity": severity.as_str(),
                })),
            )
            .await?;

        // 7. Request trust-score recalculation for affected parties.
        self.recalc
            .request_recalculation(dispute.raised_by_party_id)
            .await?;
        if let Some(against_party_id) = dispute.against_party_id {
            self.recalc.request_recalculation(against_party_id).await?;
        }

        // 8. Return refreshed dispute.
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
