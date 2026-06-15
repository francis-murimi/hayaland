use crate::errors::ApplicationError;
use crate::verifications::dto::{SubmitVerificationCommand, VerificationResult};
use domain::entities::{PartyVerification, PartyVerificationType};
use domain::repositories::{PartyRepository, PartyVerificationRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

#[derive(Clone)]
pub struct SubmitVerification {
    verification_repo: Arc<dyn PartyVerificationRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl SubmitVerification {
    pub fn new(
        verification_repo: Arc<dyn PartyVerificationRepository>,
        party_repo: Arc<dyn PartyRepository>,
    ) -> Self {
        Self {
            verification_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(party_id = %cmd.target_party_id))]
    pub async fn execute(
        &self,
        cmd: SubmitVerificationCommand,
    ) -> Result<VerificationResult, ApplicationError> {
        // 1. Resolve and validate the verification type.
        let verification_type = PartyVerificationType::try_from(cmd.verification_type.as_str())?;

        // 2. Caller must be a member of the target party, unless admin.
        if !cmd.is_admin
            && !self
                .party_repo
                .is_user_member_of_party(cmd.actor_user_id, cmd.target_party_id)
                .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        // 3. Admin-reviewed types require evidence.
        if verification_type.requires_admin_review() && cmd.evidence_urls.is_empty() {
            return Err(ApplicationError::Validation(vec![
                "verification evidence is required".to_string(),
            ]));
        }

        // 4. No duplicate active verification for the same party and type.
        if self
            .verification_repo
            .find_active_by_party_and_type(cmd.target_party_id, verification_type)
            .await?
            .is_some()
        {
            return Err(ApplicationError::DuplicateVerification);
        }

        // 5. Build and persist the verification.
        let verification = PartyVerification::new(
            Uuid::now_v7(),
            cmd.target_party_id,
            cmd.actor_user_id,
            verification_type,
            cmd.evidence_urls,
            cmd.notes,
        );

        self.verification_repo.create(&verification).await?;

        info!(
            verification_id = %verification.id,
            party_id = %verification.party_id,
            verification_type = %verification.verification_type.as_str(),
            "verification submitted"
        );

        Ok(verification.into())
    }
}
