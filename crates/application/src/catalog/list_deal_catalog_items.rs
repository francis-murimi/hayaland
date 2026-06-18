use crate::catalog::dto::{CatalogListResult, EnhancementResult, NeedResult, ResourceResult};
use crate::catalog::mappers::{
    map_enhancement_to_result, map_need_to_result, map_resource_to_result,
};
use crate::errors::ApplicationError;
use domain::repositories::CatalogRepository;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// List catalogue items that have been bound to a specific deal.
#[derive(Clone)]
pub struct ListDealCatalogItems {
    catalog_repo: Arc<dyn CatalogRepository>,
}

impl ListDealCatalogItems {
    pub fn new(catalog_repo: Arc<dyn CatalogRepository>) -> Self {
        Self { catalog_repo }
    }

    #[instrument(skip(self), fields(deal_id = %deal_id))]
    pub async fn list_resources(
        &self,
        deal_id: Uuid,
    ) -> Result<CatalogListResult<ResourceResult>, ApplicationError> {
        let items = self.catalog_repo.find_resources_by_deal(deal_id).await?;
        let total = items.len() as i64;
        info!(%total, "listed deal-bound resources");
        Ok(CatalogListResult {
            items: items.into_iter().map(map_resource_to_result).collect(),
            total,
            limit: total,
            offset: 0,
        })
    }

    #[instrument(skip(self), fields(deal_id = %deal_id))]
    pub async fn list_needs(
        &self,
        deal_id: Uuid,
    ) -> Result<CatalogListResult<NeedResult>, ApplicationError> {
        let items = self.catalog_repo.find_needs_by_deal(deal_id).await?;
        let total = items.len() as i64;
        Ok(CatalogListResult {
            items: items.into_iter().map(map_need_to_result).collect(),
            total,
            limit: total,
            offset: 0,
        })
    }

    #[instrument(skip(self), fields(deal_id = %deal_id))]
    pub async fn list_enhancements(
        &self,
        deal_id: Uuid,
    ) -> Result<CatalogListResult<EnhancementResult>, ApplicationError> {
        let items = self.catalog_repo.find_enhancements_by_deal(deal_id).await?;
        let total = items.len() as i64;
        Ok(CatalogListResult {
            items: items.into_iter().map(map_enhancement_to_result).collect(),
            total,
            limit: total,
            offset: 0,
        })
    }
}
