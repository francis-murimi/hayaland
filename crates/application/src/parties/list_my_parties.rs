use crate::errors::ApplicationError;
use crate::parties::dto::PartySummaryResult;
use domain::repositories::PartyRepository;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

/// List all parties associated with the authenticated user.
#[derive(Clone)]
pub struct ListMyParties {
    repo: Arc<dyn PartyRepository>,
}

impl ListMyParties {
    pub fn new(repo: Arc<dyn PartyRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self), fields(user_id = %user_id))]
    pub async fn execute(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PartySummaryResult>, ApplicationError> {
        let memberships = self.repo.list_memberships_for_user(user_id).await?;
        Ok(memberships
            .into_iter()
            .map(|(_, party)| PartySummaryResult {
                id: party.id,
                party_type: party.party_type,
                display_name: party.display_name.as_str().to_owned(),
                email: party.email.as_str().to_owned(),
                verification_status: party.verification_status,
                primary_domain_id: party.primary_domain_id,
                trust_score: party.trust_score,
                is_active: party.is_active,
            })
            .collect())
    }
}
