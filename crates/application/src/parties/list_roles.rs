use crate::errors::ApplicationError;
use crate::parties::dto::RoleResult;
use domain::repositories::PartyRepository;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

/// List roles assigned to a party.
#[derive(Clone)]
pub struct ListPartyRoles {
    repo: Arc<dyn PartyRepository>,
}

impl ListPartyRoles {
    pub fn new(repo: Arc<dyn PartyRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self), fields(party_id = %party_id))]
    pub async fn execute(&self, party_id: Uuid) -> Result<Vec<RoleResult>, ApplicationError> {
        // Ensure party exists.
        let _ = self
            .repo
            .find_by_id(party_id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;

        let roles = self.repo.list_roles(party_id).await?;
        Ok(roles
            .into_iter()
            .map(|(role_type, profile)| RoleResult {
                role_type,
                profile,
                is_active: true,
                assigned_at: time::OffsetDateTime::now_utc(),
            })
            .collect())
    }
}
