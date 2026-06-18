use crate::catalog::access::require_catalog_owner_or_admin;
use crate::catalog::dto::DeleteCatalogItemCommand;
use crate::errors::ApplicationError;
use domain::repositories::{CatalogRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Delete a catalogue resource. Owner or admin only. Hard-deleted when there are no
/// active deals, otherwise the repository raises `CatalogItemHasActiveDeals`.
#[derive(Clone)]
pub struct DeleteResource {
    catalog_repo: Arc<dyn CatalogRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl DeleteResource {
    pub fn new(
        catalog_repo: Arc<dyn CatalogRepository>,
        party_repo: Arc<dyn PartyRepository>,
    ) -> Self {
        Self {
            catalog_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(resource_id = %id))]
    pub async fn execute(
        &self,
        id: Uuid,
        cmd: DeleteCatalogItemCommand,
    ) -> Result<(), ApplicationError> {
        let resource = self
            .catalog_repo
            .find_resource_by_id(id)
            .await?
            .ok_or(ApplicationError::ResourceNotFound)?;

        require_catalog_owner_or_admin(
            &self.party_repo,
            cmd.actor_user_id,
            cmd.actor_party_id,
            resource.supplier_party_id,
            cmd.is_admin,
        )
        .await?;

        self.catalog_repo.delete_resource(id).await?;

        info!(%id, "deleted resource");
        Ok(())
    }
}
