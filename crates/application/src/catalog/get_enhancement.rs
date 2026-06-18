use crate::catalog::dto::{EnhancementPublicResult, EnhancementResult};
use crate::catalog::mappers::{map_enhancement_to_public, map_enhancement_to_result};
use crate::errors::ApplicationError;
use domain::repositories::CatalogRepository;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Get an enhancement by ID, returning either the full owner/admin view or the public view.
#[derive(Clone)]
pub struct GetEnhancement {
    catalog_repo: Arc<dyn CatalogRepository>,
}

impl GetEnhancement {
    pub fn new(catalog_repo: Arc<dyn CatalogRepository>) -> Self {
        Self { catalog_repo }
    }

    #[instrument(skip(self), fields(enhancement_id = %id))]
    pub async fn execute(
        &self,
        id: Uuid,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<EnhancementView, ApplicationError> {
        let enhancement = self
            .catalog_repo
            .find_enhancement_by_id(id)
            .await?
            .ok_or(ApplicationError::EnhancementNotFound)?;

        if !enhancement.is_visible_to(actor_party_id, is_admin) {
            return Err(ApplicationError::EnhancementNotFound);
        }

        info!(%id, "fetched enhancement");

        let is_owner = actor_party_id == Some(enhancement.enhancer_party_id);
        if is_admin || is_owner {
            Ok(EnhancementView::Owner(map_enhancement_to_result(
                enhancement,
            )))
        } else {
            Ok(EnhancementView::Public(map_enhancement_to_public(
                enhancement,
            )))
        }
    }
}

#[derive(Debug, Clone)]
pub enum EnhancementView {
    Owner(EnhancementResult),
    Public(EnhancementPublicResult),
}
