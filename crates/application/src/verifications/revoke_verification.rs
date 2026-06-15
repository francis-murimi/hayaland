use crate::errors::ApplicationError;
use crate::ports::TrustScoreRecalculationPort;
use crate::verifications::dto::{
    party_verification_status_for_points, RevokeVerificationCommand, VerificationResult,
};
use domain::entities::verification_level_from_points;
use domain::repositories::{PartyRepository, PartyVerificationRepository};
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Clone)]
pub struct RevokeVerification {
    verification_repo: Arc<dyn PartyVerificationRepository>,
    party_repo: Arc<dyn PartyRepository>,
    recalc: Arc<dyn TrustScoreRecalculationPort>,
}

impl RevokeVerification {
    pub fn new(
        verification_repo: Arc<dyn PartyVerificationRepository>,
        party_repo: Arc<dyn PartyRepository>,
        recalc: Arc<dyn TrustScoreRecalculationPort>,
    ) -> Self {
        Self {
            verification_repo,
            party_repo,
            recalc,
        }
    }

    #[instrument(skip(self, cmd), fields(verification_id = %cmd.verification_id))]
    pub async fn execute(
        &self,
        cmd: RevokeVerificationCommand,
    ) -> Result<VerificationResult, ApplicationError> {
        // 1. Revoke the verification record.
        self.verification_repo
            .revoke(
                cmd.verification_id,
                cmd.actor_user_id,
                cmd.reason,
                cmd.review_notes,
            )
            .await?;

        // 2. Load the verification to know the affected party.
        let verification = self
            .verification_repo
            .find_by_id(cmd.verification_id)
            .await?
            .ok_or(ApplicationError::VerificationNotFound)?;

        // 3. Synchronize the high-level party verification status.
        self.sync_party_status(verification.party_id).await?;

        info!(
            verification_id = %cmd.verification_id,
            party_id = %verification.party_id,
            "verification revoked"
        );

        // 4. Trigger trust-score recalculation.
        self.recalc
            .request_recalculation(verification.party_id)
            .await?;

        Ok(verification.into())
    }

    async fn sync_party_status(&self, party_id: uuid::Uuid) -> Result<(), ApplicationError> {
        let effective_points = self.verification_repo.sum_approved_points(party_id).await?;
        let pending_count = self
            .verification_repo
            .count_by_status(party_id, "PENDING")
            .await?;
        let new_status = party_verification_status_for_points(effective_points, pending_count);

        let mut party = self
            .party_repo
            .find_by_id(party_id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;

        if party.verification_status != new_status {
            party.verification_status = new_status;
            party.updated_at = time::OffsetDateTime::now_utc();
            self.party_repo.update(&party).await?;
        }

        let level = verification_level_from_points(effective_points);
        self.verification_repo
            .update_verification_level(party_id, level)
            .await?;

        Ok(())
    }
}
