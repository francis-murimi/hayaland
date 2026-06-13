use crate::errors::ApplicationError;
use crate::parties::dto::{AddPartyRoleCommand, RoleResult};
use domain::repositories::PartyRepository;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

/// Add a role to a party.
#[derive(Clone)]
pub struct AddPartyRole {
    repo: Arc<dyn PartyRepository>,
}

impl AddPartyRole {
    pub fn new(repo: Arc<dyn PartyRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self, cmd), fields(party_id = %party_id))]
    pub async fn execute(
        &self,
        party_id: Uuid,
        cmd: AddPartyRoleCommand,
    ) -> Result<RoleResult, ApplicationError> {
        if !cmd.is_admin {
            let membership = self
                .repo
                .find_membership(cmd.actor_user_id, party_id)
                .await?;
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

        self.repo
            .add_role(party_id, cmd.role, cmd.profile.clone())
            .await?;

        Ok(RoleResult {
            role_type: cmd.role,
            profile: cmd.profile,
            is_active: true,
            assigned_at: time::OffsetDateTime::now_utc(),
        })
    }
}
