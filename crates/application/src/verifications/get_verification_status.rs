use crate::errors::ApplicationError;
use crate::verifications::dto::{
    next_level_points, party_verification_status_for_points, GetVerificationStatusQuery,
    VerificationStatusResult,
};
use domain::entities::verification_level_from_points;
use domain::repositories::{PartyRepository, PartyVerificationRepository};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct GetVerificationStatus {
    verification_repo: Arc<dyn PartyVerificationRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl GetVerificationStatus {
    pub fn new(
        verification_repo: Arc<dyn PartyVerificationRepository>,
        party_repo: Arc<dyn PartyRepository>,
    ) -> Self {
        Self {
            verification_repo,
            party_repo,
        }
    }

    pub async fn execute(
        &self,
        party_id: Uuid,
        query: GetVerificationStatusQuery,
    ) -> Result<VerificationStatusResult, ApplicationError> {
        // Caller must be a member of the party, unless admin.
        if !query.is_admin
            && !self
                .party_repo
                .is_user_member_of_party(query.actor_user_id, party_id)
                .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        let effective_points = self.verification_repo.sum_approved_points(party_id).await?;
        let pending_count = self
            .verification_repo
            .count_by_status(party_id, "PENDING")
            .await?;
        let approved_count = self
            .verification_repo
            .count_by_status(party_id, "APPROVED")
            .await?;
        let rejected_count = self
            .verification_repo
            .count_by_status(party_id, "REJECTED")
            .await?;
        let revoked_count = self
            .verification_repo
            .count_by_status(party_id, "REVOKED")
            .await?;
        let expired_count = self
            .verification_repo
            .count_by_status(party_id, "EXPIRED")
            .await?;

        let verification_level = verification_level_from_points(effective_points);
        let verification_status =
            party_verification_status_for_points(effective_points, pending_count);

        Ok(VerificationStatusResult {
            party_id,
            verification_status: verification_status.as_str().to_string(),
            verification_level,
            effective_points,
            pending_count,
            approved_count,
            rejected_count,
            revoked_count,
            expired_count,
            next_level_points: next_level_points(effective_points),
        })
    }
}
