use crate::catalog::dto::{
    CatalogListResult, CatalogSearchQuery, NeedPublicResult, NeedSummaryResult,
};
use crate::catalog::list_resources::build_criteria;
use crate::catalog::mappers::{map_need_to_public, map_need_to_summary};
use crate::errors::ApplicationError;
use domain::repositories::CatalogRepository;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// List consumer needs using public visibility rules.
#[derive(Clone)]
pub struct ListNeeds {
    catalog_repo: Arc<dyn CatalogRepository>,
}

impl ListNeeds {
    pub fn new(catalog_repo: Arc<dyn CatalogRepository>) -> Self {
        Self { catalog_repo }
    }

    #[instrument(skip(self, query))]
    pub async fn execute(
        &self,
        query: CatalogSearchQuery,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<CatalogListResult<NeedPublicResult>, ApplicationError> {
        let criteria = build_criteria(&query, actor_party_id, is_admin)?;

        let result = self.catalog_repo.list_needs(&criteria).await?;

        info!(total = %result.total, "listed needs");

        Ok(CatalogListResult {
            items: result.items.into_iter().map(map_need_to_public).collect(),
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
    ) -> Result<CatalogListResult<NeedSummaryResult>, ApplicationError> {
        let criteria = build_criteria(&query, actor_party_id, is_admin)?;

        let result = self.catalog_repo.list_needs(&criteria).await?;

        Ok(CatalogListResult {
            items: result.items.into_iter().map(map_need_to_summary).collect(),
            total: result.total,
            limit: result.limit,
            offset: result.offset,
        })
    }
}
