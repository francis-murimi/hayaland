use crate::errors::ApplicationError;
use domain::repositories::PartyRepository;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

/// Soft-delete a party. Admins can force delete even with active deals (future enhancement).
#[derive(Clone)]
pub struct SoftDeleteParty {
    repo: Arc<dyn PartyRepository>,
}

impl SoftDeleteParty {
    pub fn new(repo: Arc<dyn PartyRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self), fields(party_id = %party_id))]
    pub async fn execute(
        &self,
        party_id: Uuid,
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
                        ) => {}
                _ => return Err(ApplicationError::Forbidden),
            }

            let active_deals = self.repo.count_active_deals(party_id).await?;
            if active_deals > 0 {
                return Err(ApplicationError::PartyHasActiveDeals);
            }
        }

        self.repo.soft_delete(party_id).await?;
        Ok(())
    }
}
