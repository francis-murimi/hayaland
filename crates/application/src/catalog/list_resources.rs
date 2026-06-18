use crate::catalog::dto::{
    CatalogListResult, CatalogSearchQuery, ResourcePublicResult, ResourceSummaryResult,
};
use crate::catalog::mappers::{map_resource_to_public, map_resource_to_summary};
use crate::errors::ApplicationError;
use domain::repositories::{
    CatalogItemStatus, CatalogRepository, CatalogSearchCriteria, CatalogSort, GeoSearch,
};
use std::sync::Arc;
use tracing::{info, instrument};

/// List resources using public visibility rules. If the caller filters by their own
/// party ID, hidden and inactive owned items are included.
#[derive(Clone)]
pub struct ListResources {
    catalog_repo: Arc<dyn CatalogRepository>,
}

impl ListResources {
    pub fn new(catalog_repo: Arc<dyn CatalogRepository>) -> Self {
        Self { catalog_repo }
    }

    #[instrument(skip(self, query))]
    pub async fn execute(
        &self,
        query: CatalogSearchQuery,
        actor_party_id: Option<uuid::Uuid>,
        is_admin: bool,
    ) -> Result<CatalogListResult<ResourcePublicResult>, ApplicationError> {
        let criteria = build_criteria(&query, actor_party_id, is_admin)?;

        let result = self.catalog_repo.list_resources(&criteria).await?;

        info!(total = %result.total, "listed resources");

        Ok(CatalogListResult {
            items: result
                .items
                .into_iter()
                .map(map_resource_to_public)
                .collect(),
            total: result.total,
            limit: result.limit,
            offset: result.offset,
        })
    }

    /// Owner/admin list variant that returns full summary data including hidden flags.
    #[instrument(skip(self, query))]
    pub async fn execute_summary(
        &self,
        query: CatalogSearchQuery,
        actor_party_id: Option<uuid::Uuid>,
        is_admin: bool,
    ) -> Result<CatalogListResult<ResourceSummaryResult>, ApplicationError> {
        let criteria = build_criteria(&query, actor_party_id, is_admin)?;

        let result = self.catalog_repo.list_resources(&criteria).await?;

        Ok(CatalogListResult {
            items: result
                .items
                .into_iter()
                .map(map_resource_to_summary)
                .collect(),
            total: result.total,
            limit: result.limit,
            offset: result.offset,
        })
    }
}

pub(crate) fn build_criteria(
    query: &CatalogSearchQuery,
    actor_party_id: Option<uuid::Uuid>,
    is_admin: bool,
) -> Result<CatalogSearchCriteria, ApplicationError> {
    let is_owner_view = query.party_id.is_some() && query.party_id == actor_party_id;
    let include_hidden = is_admin || is_owner_view;
    let include_inactive = is_admin || is_owner_view;

    let status = match query.status.as_deref() {
        Some(s) => Some(CatalogItemStatus::try_from(s).map_err(ApplicationError::from)?),
        None => {
            if include_inactive {
                Some(CatalogItemStatus::All)
            } else {
                Some(CatalogItemStatus::Active)
            }
        }
    };

    let sort = match query.sort.as_deref() {
        Some(s) => CatalogSort::try_from(s).map_err(ApplicationError::from)?,
        None => CatalogSort::Newest,
    };

    let geo = match (query.latitude, query.longitude, query.radius_km) {
        (Some(lat), Some(lng), Some(radius)) => Some(GeoSearch {
            latitude: lat,
            longitude: lng,
            radius_km: radius,
        }),
        _ => None,
    };

    Ok(CatalogSearchCriteria {
        party_id: query.party_id,
        category_id: query.category_id,
        domain_category_id: query.domain_category_id,
        query: query.text.clone(),
        status,
        geo,
        verified_only: query.verified_only,
        featured_only: query.featured_only,
        include_hidden,
        include_inactive,
        sort,
        limit: query.limit,
        offset: query.offset,
    })
}
