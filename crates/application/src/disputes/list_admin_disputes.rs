use crate::disputes::dto::{AdminDisputeListQuery, DisputeListResult};
use crate::errors::ApplicationError;
use domain::entities::DisputeStatus;
use domain::repositories::{DisputeFilters, DisputeRepository};
use std::sync::Arc;
use tracing::instrument;

#[derive(Clone)]
pub struct ListAdminDisputes {
    dispute_repo: Arc<dyn DisputeRepository>,
}

impl ListAdminDisputes {
    pub fn new(dispute_repo: Arc<dyn DisputeRepository>) -> Self {
        Self { dispute_repo }
    }

    #[instrument(skip(self, query))]
    pub async fn execute(
        &self,
        query: AdminDisputeListQuery,
    ) -> Result<DisputeListResult, ApplicationError> {
        let status = query
            .status
            .map(|s| DisputeStatus::try_from(s.as_str()))
            .transpose()
            .map_err(ApplicationError::from)?;

        let filters = DisputeFilters {
            status,
            deal_id: query.deal_id,
            raised_by_party_id: query.raised_by_party_id,
            against_party_id: query.against_party_id,
            limit: query.limit,
            offset: query.offset,
        };

        let result = self.dispute_repo.list_admin(&filters).await?;

        Ok(DisputeListResult {
            disputes: result.disputes.into_iter().map(Into::into).collect(),
            total: result.total,
            limit: result.limit,
            offset: result.offset,
        })
    }
}
