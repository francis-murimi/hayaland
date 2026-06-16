use crate::disputes::dto::{DisputeResult, RaiseDisputeCommand};
use crate::errors::ApplicationError;
use crate::ports::TrustScoreRecalculationPort;
use domain::entities::{DealStatus, Dispute, DisputeType};
use domain::repositories::{DealRepository, DisputeRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

#[derive(Clone)]
pub struct RaiseDispute {
    dispute_repo: Arc<dyn DisputeRepository>,
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
    recalc: Arc<dyn TrustScoreRecalculationPort>,
}

impl RaiseDispute {
    pub fn new(
        dispute_repo: Arc<dyn DisputeRepository>,
        deal_repo: Arc<dyn DealRepository>,
        party_repo: Arc<dyn PartyRepository>,
        recalc: Arc<dyn TrustScoreRecalculationPort>,
    ) -> Self {
        Self {
            dispute_repo,
            deal_repo,
            party_repo,
            recalc,
        }
    }

    #[instrument(skip(self, cmd), fields(deal_id = %cmd.deal_id))]
    pub async fn execute(
        &self,
        cmd: RaiseDisputeCommand,
    ) -> Result<DisputeResult, ApplicationError> {
        // 1. Load deal aggregate.
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(cmd.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        let mut deal = aggregate.deal;
        let participations = aggregate.participations;

        // 2. Only active, non-terminal deals can be disputed.
        if deal.deal_status != DealStatus::Executing && deal.deal_status != DealStatus::OnHold {
            return Err(ApplicationError::InvalidStateTransition {
                from: deal.deal_status.as_str().to_string(),
                to: DealStatus::Disputed.as_str().to_string(),
            });
        }

        // 3. Caller must be a member of the acting party, unless admin.
        if !cmd.is_admin
            && !self
                .party_repo
                .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
                .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        // 4. Acting party must participate in the deal.
        if !participations
            .iter()
            .any(|p| p.party_id == cmd.actor_party_id)
        {
            return Err(ApplicationError::DealAccessDenied);
        }

        // 5. If an against party is supplied, it must be a different participant.
        if let Some(against_party_id) = cmd.against_party_id {
            if against_party_id == cmd.actor_party_id {
                return Err(ApplicationError::Validation(vec![
                    "cannot raise a dispute against yourself".to_string(),
                ]));
            }
            if !participations
                .iter()
                .any(|p| p.party_id == against_party_id)
            {
                return Err(ApplicationError::DealAccessDenied);
            }
        }

        // 6. Parse dispute type.
        let dispute_type =
            DisputeType::try_from(cmd.dispute_type.as_str()).map_err(ApplicationError::from)?;

        // 7. Create the dispute.
        let dispute = Dispute::new(
            Uuid::now_v7(),
            cmd.deal_id,
            cmd.actor_party_id,
            cmd.actor_user_id,
            cmd.against_party_id,
            dispute_type,
            cmd.description,
            cmd.evidence_urls,
        );

        // 8. Transition deal to DISPUTED.
        deal.transition(DealStatus::Disputed)?;

        // 9. Persist.
        self.dispute_repo.create(&dispute).await?;
        self.deal_repo.update(&deal).await?;
        self.deal_repo
            .record_history(
                cmd.deal_id,
                "DISPUTE_RAISED",
                Some(cmd.actor_party_id),
                Some(serde_json::json!({"dispute_id": dispute.id})),
            )
            .await?;

        // 10. Update trust-score dispute counts and request recalculation.
        self.dispute_repo
            .increment_deals_disputed_count(cmd.actor_party_id)
            .await?;
        self.recalc
            .request_recalculation(cmd.actor_party_id)
            .await?;
        if let Some(against_party_id) = cmd.against_party_id {
            self.dispute_repo
                .increment_deals_disputed_count(against_party_id)
                .await?;
            self.recalc.request_recalculation(against_party_id).await?;
        }

        info!(
            dispute_id = %dispute.id,
            deal_id = %cmd.deal_id,
            raised_by_party_id = %cmd.actor_party_id,
            "dispute raised"
        );

        Ok(dispute.into())
    }
}
