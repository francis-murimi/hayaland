use crate::disputes::dto::{DisputeResult, GetDisputeQuery};
use crate::errors::ApplicationError;
use domain::repositories::{DealRepository, DisputeRepository};
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

#[derive(Clone)]
pub struct GetDispute {
    deal_repo: Arc<dyn DealRepository>,
    dispute_repo: Arc<dyn DisputeRepository>,
}

impl GetDispute {
    pub fn new(
        deal_repo: Arc<dyn DealRepository>,
        dispute_repo: Arc<dyn DisputeRepository>,
    ) -> Self {
        Self {
            deal_repo,
            dispute_repo,
        }
    }

    #[instrument(skip(self, query), fields(dispute_id = %query.dispute_id))]
    pub async fn execute(
        &self,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
        query: GetDisputeQuery,
    ) -> Result<DisputeResult, ApplicationError> {
        // 1. Load dispute.
        let dispute = self
            .dispute_repo
            .find_by_id(query.dispute_id)
            .await?
            .ok_or(ApplicationError::DisputeNotFound)?;

        // 2. Load deal aggregate.
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(dispute.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        // 3. Caller must be a participant or admin.
        let visible = if is_admin {
            true
        } else if let Some(party_id) = actor_party_id {
            aggregate
                .participations
                .iter()
                .any(|p| p.party_id == party_id)
        } else {
            false
        };

        if !visible {
            return Err(ApplicationError::DealAccessDenied);
        }

        // 4. Load responses.
        let responses = self.dispute_repo.list_responses(query.dispute_id).await?;

        let mut result: DisputeResult = dispute.into();
        result.responses = responses.into_iter().map(Into::into).collect();

        Ok(result)
    }
}
