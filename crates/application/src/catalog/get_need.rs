use crate::catalog::dto::{NeedPublicResult, NeedResult};
use crate::catalog::mappers::{map_need_to_public, map_need_to_result};
use crate::errors::ApplicationError;
use domain::repositories::CatalogRepository;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Get a need by ID, returning either the full owner/admin view or the public view.
#[derive(Clone)]
pub struct GetNeed {
    catalog_repo: Arc<dyn CatalogRepository>,
}

impl GetNeed {
    pub fn new(catalog_repo: Arc<dyn CatalogRepository>) -> Self {
        Self { catalog_repo }
    }

    #[instrument(skip(self), fields(need_id = %id))]
    pub async fn execute(
        &self,
        id: Uuid,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<NeedView, ApplicationError> {
        let need = self
            .catalog_repo
            .find_need_by_id(id)
            .await?
            .ok_or(ApplicationError::NeedNotFound)?;

        if !need.is_visible_to(actor_party_id, is_admin) {
            return Err(ApplicationError::NeedNotFound);
        }

        info!(%id, "fetched need");

        let is_owner = actor_party_id == Some(need.consumer_party_id);
        if is_admin || is_owner {
            Ok(NeedView::Owner(map_need_to_result(need)))
        } else {
            Ok(NeedView::Public(map_need_to_public(need)))
        }
    }
}

#[derive(Debug, Clone)]
pub enum NeedView {
    Owner(NeedResult),
    Public(NeedPublicResult),
}
