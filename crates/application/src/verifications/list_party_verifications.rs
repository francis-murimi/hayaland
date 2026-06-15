use crate::errors::ApplicationError;
use crate::verifications::dto::{ListPartyVerificationsQuery, VerificationResult};
use domain::repositories::{PartyRepository, PartyVerificationRepository};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ListPartyVerifications {
    verification_repo: Arc<dyn PartyVerificationRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl ListPartyVerifications {
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
        query: ListPartyVerificationsQuery,
    ) -> Result<Vec<VerificationResult>, ApplicationError> {
        // Caller must be a member of the party, unless admin.
        let is_member = self
            .party_repo
            .is_user_member_of_party(query.actor_user_id, party_id)
            .await?;
        if !query.is_admin && !is_member {
            return Err(ApplicationError::Forbidden);
        }

        let verifications = self.verification_repo.list_by_party(party_id).await?;

        // Only members and admins see evidence URLs.
        let include_evidence = query.is_admin || is_member;

        Ok(verifications
            .into_iter()
            .map(|mut v| {
                if !include_evidence {
                    v.evidence_urls.clear();
                }
                VerificationResult::from(v)
            })
            .collect())
    }
}
