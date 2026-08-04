use crate::errors::ApplicationError;
use crate::notifications::LifecycleNotifier;
use crate::ports::TrustScoreRecalculationPort;
use crate::verifications::dto::{
    party_verification_status_for_points, ApproveVerificationCommand, VerificationResult,
};
use domain::entities::{verification_level_from_points, NotificationType};
use domain::repositories::{PartyRepository, PartyVerificationRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

#[derive(Clone)]
pub struct ApproveVerification {
    verification_repo: Arc<dyn PartyVerificationRepository>,
    party_repo: Arc<dyn PartyRepository>,
    recalc: Arc<dyn TrustScoreRecalculationPort>,
    notifier: Option<Arc<LifecycleNotifier>>,
}

impl ApproveVerification {
    pub fn new(
        verification_repo: Arc<dyn PartyVerificationRepository>,
        party_repo: Arc<dyn PartyRepository>,
        recalc: Arc<dyn TrustScoreRecalculationPort>,
    ) -> Self {
        Self {
            verification_repo,
            party_repo,
            recalc,
            notifier: None,
        }
    }

    pub fn with_notifier(mut self, notifier: Arc<LifecycleNotifier>) -> Self {
        self.notifier = Some(notifier);
        self
    }

    #[instrument(skip(self, cmd), fields(verification_id = %cmd.verification_id))]
    pub async fn execute(
        &self,
        cmd: ApproveVerificationCommand,
    ) -> Result<VerificationResult, ApplicationError> {
        // 1. Approve the verification record.
        self.verification_repo
            .approve(cmd.verification_id, cmd.actor_user_id, cmd.review_notes)
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
            "verification approved"
        );

        self.emit_verification_notification(
            verification.party_id,
            verification.id,
            cmd.actor_user_id,
            NotificationType::VerificationApproved,
        )
        .await;

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

        // Upsert the verification level into trust_scores for consumers that read it directly.
        let level = verification_level_from_points(effective_points);
        self.verification_repo
            .update_verification_level(party_id, level)
            .await?;

        Ok(())
    }

    async fn emit_verification_notification(
        &self,
        party_id: Uuid,
        verification_id: Uuid,
        actor_user_id: Uuid,
        notification_type: NotificationType,
    ) {
        let Some(notifier) = self.notifier.as_ref() else {
            return;
        };
        let metadata = serde_json::json!({
            "verification_id": verification_id,
        });
        let result = notifier
            .notify_party_members(
                actor_user_id,
                party_id,
                notification_type,
                Some("verification"),
                Some(verification_id),
                metadata,
            )
            .await;
        notifier.fire_and_forget(result, "verification status changed");
    }
}
