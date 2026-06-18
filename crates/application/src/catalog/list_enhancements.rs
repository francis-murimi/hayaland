use crate::catalog::dto::{
    CatalogListResult, CatalogSearchQuery, EnhancementPublicResult, EnhancementSummaryResult,
};
use crate::catalog::list_resources::build_criteria;
use crate::catalog::mappers::{map_enhancement_to_public, map_enhancement_to_summary};
use crate::errors::ApplicationError;
use domain::repositories::CatalogRepository;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// List enhancer enhancements using public visibility rules.
#[derive(Clone)]
pub struct ListEnhancements {
    catalog_repo: Arc<dyn CatalogRepository>,
}

impl ListEnhancements {
    pub fn new(catalog_repo: Arc<dyn CatalogRepository>) -> Self {
        Self { catalog_repo }
    }

    #[instrument(skip(self, query))]
    pub async fn execute(
        &self,
        query: CatalogSearchQuery,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<CatalogListResult<EnhancementPublicResult>, ApplicationError> {
        let criteria = build_criteria(&query, actor_party_id, is_admin)?;

        let result = self.catalog_repo.list_enhancements(&criteria).await?;

        info!(total = %result.total, "listed enhancements");

        Ok(CatalogListResult {
            items: result
                .items
                .into_iter()
                .map(map_enhancement_to_public)
                .collect(),
            total: result.total,
            limit: result.limit,
            offset: result.offset,
        })
    }

    /// Owner/admin list variant that returns full summary data.
    #[instrument(skip(self, query))]
    pub async fn execute_summary(
        &self,
        query: CatalogSearchQuery,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<CatalogListResult<EnhancementSummaryResult>, ApplicationError> {
        let criteria = build_criteria(&query, actor_party_id, is_admin)?;

        let result = self.catalog_repo.list_enhancements(&criteria).await?;

        Ok(CatalogListResult {
            items: result
                .items
                .into_iter()
                .map(map_enhancement_to_summary)
                .collect(),
            total: result.total,
            limit: result.limit,
            offset: result.offset,
        })
    }
}
