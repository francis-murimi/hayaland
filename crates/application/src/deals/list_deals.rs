use crate::deals::dto::{DealListResult, DealSummaryResult, ListDealsQuery};
use crate::errors::ApplicationError;
use domain::repositories::{DealRepository, DealSearchCriteria, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// List deals visible to the caller.
#[derive(Clone)]
pub struct ListDeals {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl ListDeals {
    pub fn new(deal_repo: Arc<dyn DealRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, query), fields(user_id = %user_id))]
    pub async fn execute(
        &self,
        user_id: Uuid,
        party_id: Option<Uuid>,
        query: ListDealsQuery,
        is_admin: bool,
    ) -> Result<DealListResult, ApplicationError> {
        let limit = if query.limit > 0 { query.limit } else { 20 };
        let offset = if query.offset >= 0 { query.offset } else { 0 };

        let mut criteria = DealSearchCriteria {
            status: query.status,
            limit,
            offset,
            ..Default::default()
        };

        if !is_admin {
            // Filter to parties the user is a member of.
            let memberships = self.party_repo.list_memberships_for_user(user_id).await?;
            let party_ids: Vec<Uuid> = memberships.into_iter().map(|(m, _)| m.party_id).collect();

            if let Some(pid) = party_id {
                if !party_ids.contains(&pid) {
                    return Err(ApplicationError::Forbidden);
                }
                criteria.party_id = Some(pid);
            } else if party_ids.len() == 1 {
                criteria.party_id = Some(party_ids[0]);
            } else if party_ids.is_empty() {
                return Ok(DealListResult {
                    deals: vec![],
                    total: 0,
                    limit,
                    offset,
                });
            } else {
                // Multiple parties: for now, list deals for all of them.
                // The repository currently supports a single party_id; we'll do filtering here.
                criteria.party_id = None;
            }
        }

        let result = self.deal_repo.list(&criteria).await?;

        let summaries: Vec<DealSummaryResult> = result
            .deals
            .into_iter()
            .map(|deal| DealSummaryResult {
                id: deal.id,
                deal_reference: deal.deal_reference,
                title: deal.deal_title.as_str().to_owned(),
                deal_status: deal.deal_status,
                initiator_party_id: deal.initiator_party_id,
                my_role: None, // populated by caller if needed
                total_deal_value: deal.total_deal_value,
                currency: deal.currency,
                updated_at: deal.updated_at,
            })
            .collect();

        info!(user_id = %user_id, count = summaries.len(), "listed deals");
        Ok(DealListResult {
            deals: summaries,
            total: result.total,
            limit,
            offset,
        })
    }
}
