use crate::catalog::dto::{ResourcePublicResult, ResourceResult};
use crate::catalog::mappers::{map_resource_to_public, map_resource_to_result};
use crate::errors::ApplicationError;
use domain::repositories::CatalogRepository;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Get a resource by ID, returning either the full owner/admin view or the public view.
#[derive(Clone)]
pub struct GetResource {
    catalog_repo: Arc<dyn CatalogRepository>,
}

impl GetResource {
    pub fn new(catalog_repo: Arc<dyn CatalogRepository>) -> Self {
        Self { catalog_repo }
    }

    #[instrument(skip(self), fields(resource_id = %id))]
    pub async fn execute(
        &self,
        id: Uuid,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<ResourceView, ApplicationError> {
        let resource = self
            .catalog_repo
            .find_resource_by_id(id)
            .await?
            .ok_or(ApplicationError::ResourceNotFound)?;

        if !resource.is_visible_to(actor_party_id, is_admin) {
            return Err(ApplicationError::ResourceNotFound);
        }

        info!(%id, "fetched resource");

        let is_owner = actor_party_id == Some(resource.supplier_party_id);
        if is_admin || is_owner {
            Ok(ResourceView::Owner(map_resource_to_result(resource)))
        } else {
            Ok(ResourceView::Public(map_resource_to_public(resource)))
        }
    }
}

#[derive(Debug, Clone)]
pub enum ResourceView {
    Owner(ResourceResult),
    Public(ResourcePublicResult),
}
