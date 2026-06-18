use crate::catalog::access::require_catalog_owner_or_admin;
use crate::catalog::dto::DeleteCatalogItemCommand;
use crate::errors::ApplicationError;
use domain::repositories::{CatalogRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Delete a catalogue enhancement. Owner or admin only.
#[derive(Clone)]
pub struct DeleteEnhancement {
    catalog_repo: Arc<dyn CatalogRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl DeleteEnhancement {
    pub fn new(
        catalog_repo: Arc<dyn CatalogRepository>,
        party_repo: Arc<dyn PartyRepository>,
    ) -> Self {
        Self {
            catalog_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(enhancement_id = %id))]
    pub async fn execute(
        &self,
        id: Uuid,
        cmd: DeleteCatalogItemCommand,
    ) -> Result<(), ApplicationError> {
        let enhancement = self
            .catalog_repo
            .find_enhancement_by_id(id)
            .await?
            .ok_or(ApplicationError::EnhancementNotFound)?;

        require_catalog_owner_or_admin(
            &self.party_repo,
            cmd.actor_user_id,
            cmd.actor_party_id,
            enhancement.enhancer_party_id,
            cmd.is_admin,
        )
        .await?;

        self.catalog_repo.delete_enhancement(id).await?;

        info!(%id, "deleted enhancement");
        Ok(())
    }
}
