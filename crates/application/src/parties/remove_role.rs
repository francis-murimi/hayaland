use crate::errors::ApplicationError;
use domain::entities::DealRole;
use domain::repositories::PartyRepository;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

/// Remove a role from a party.
#[derive(Clone)]
pub struct RemovePartyRole {
    repo: Arc<dyn PartyRepository>,
}

impl RemovePartyRole {
    pub fn new(repo: Arc<dyn PartyRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self), fields(party_id = %party_id))]
    pub async fn execute(
        &self,
        party_id: Uuid,
        role: DealRole,
        actor_user_id: Uuid,
        is_admin: bool,
    ) -> Result<(), ApplicationError> {
        if !is_admin {
            let membership = self.repo.find_membership(actor_user_id, party_id).await?;
            match membership {
                Some(m)
                    if m.is_active
                        && matches!(
                            m.member_role,
                            domain::entities::PartyMembershipRole::Owner
                                | domain::entities::PartyMembershipRole::Admin
                        ) => {}
                _ => return Err(ApplicationError::Forbidden),
            }
        }

        // Ensure party exists.
        let _ = self
            .repo
            .find_by_id(party_id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;

        self.repo.remove_role(party_id, role).await?;
        Ok(())
    }
}
