use crate::catalog::access::require_catalog_owner_or_admin;
use crate::catalog::dto::{PartyCatalogSettingsResult, UpdatePartyCatalogSettingsCommand};
use crate::errors::ApplicationError;
use domain::repositories::PartyRepository;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Update party-level catalogue settings.
#[derive(Clone)]
pub struct UpdatePartyCatalogSettings {
    party_repo: Arc<dyn PartyRepository>,
}

impl UpdatePartyCatalogSettings {
    pub fn new(party_repo: Arc<dyn PartyRepository>) -> Self {
        Self { party_repo }
    }

    #[instrument(skip(self, cmd), fields(party_id = %party_id))]
    pub async fn execute(
        &self,
        party_id: Uuid,
        cmd: UpdatePartyCatalogSettingsCommand,
    ) -> Result<PartyCatalogSettingsResult, ApplicationError> {
        let mut party = self
            .party_repo
            .find_by_id(party_id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;

        require_catalog_owner_or_admin(
            &self.party_repo,
            cmd.actor_user_id,
            cmd.actor_party_id,
            party_id,
            cmd.is_admin,
        )
        .await?;

        if let Some(flag) = cmd.accepts_catalog_inquiries {
            party.accepts_catalog_inquiries = flag;
        }
        if let Some(flag) = cmd.public_contact_email {
            party.public_contact_email = flag;
        }
        party.updated_at = time::OffsetDateTime::now_utc();

        self.party_repo.update(&party).await?;

        info!(%party_id, "updated party catalog settings");
        Ok(PartyCatalogSettingsResult {
            party_id,
            accepts_catalog_inquiries: party.accepts_catalog_inquiries,
            public_contact_email: party.public_contact_email,
        })
    }
}
