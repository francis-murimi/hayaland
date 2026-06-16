use crate::disputes::dto::{DisputeListResult, ListDealDisputesQuery};
use crate::errors::ApplicationError;
use domain::repositories::{DealRepository, DisputeRepository};
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

#[derive(Clone)]
pub struct ListDealDisputes {
    deal_repo: Arc<dyn DealRepository>,
    dispute_repo: Arc<dyn DisputeRepository>,
}

impl ListDealDisputes {
    pub fn new(
        deal_repo: Arc<dyn DealRepository>,
        dispute_repo: Arc<dyn DisputeRepository>,
    ) -> Self {
        Self {
            deal_repo,
            dispute_repo,
        }
    }

    #[instrument(skip(self, query), fields(deal_id = %query.deal_id))]
    pub async fn execute(
        &self,
        deal_id: Uuid,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
        query: ListDealDisputesQuery,
    ) -> Result<DisputeListResult, ApplicationError> {
        // 1. Load deal aggregate.
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        let participations = aggregate.participations;

        // 2. Caller must be a participant or admin.
        let visible = if is_admin {
            true
        } else if let Some(party_id) = actor_party_id {
            participations.iter().any(|p| p.party_id == party_id)
        } else {
            false
        };

        if !visible {
            return Err(ApplicationError::DealAccessDenied);
        }

        // 3. List disputes.
        let result = self
            .dispute_repo
            .list_by_deal(deal_id, query.limit.max(1), query.offset.max(0))
            .await?;

        Ok(DisputeListResult {
            disputes: result.disputes.into_iter().map(Into::into).collect(),
            total: result.total,
            limit: result.limit,
            offset: result.offset,
        })
    }
}
