use crate::catalog::access::require_catalog_owner_or_admin;
use crate::catalog::dto::DeleteCatalogItemCommand;
use crate::errors::ApplicationError;
use domain::repositories::{CatalogRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Delete a catalogue need. Owner or admin only.
#[derive(Clone)]
pub struct DeleteNeed {
    catalog_repo: Arc<dyn CatalogRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl DeleteNeed {
    pub fn new(
        catalog_repo: Arc<dyn CatalogRepository>,
        party_repo: Arc<dyn PartyRepository>,
    ) -> Self {
        Self {
            catalog_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(need_id = %id))]
    pub async fn execute(
        &self,
        id: Uuid,
        cmd: DeleteCatalogItemCommand,
    ) -> Result<(), ApplicationError> {
        let need = self
            .catalog_repo
            .find_need_by_id(id)
            .await?
            .ok_or(ApplicationError::NeedNotFound)?;

        require_catalog_owner_or_admin(
            &self.party_repo,
            cmd.actor_user_id,
            cmd.actor_party_id,
            need.consumer_party_id,
            cmd.is_admin,
        )
        .await?;

        self.catalog_repo.delete_need(id).await?;

        info!(%id, "deleted need");
        Ok(())
    }
}
