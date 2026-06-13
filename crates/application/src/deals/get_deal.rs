use crate::deals::create_deal::map_aggregate_to_result;
use crate::deals::dto::DealResult;
use crate::errors::ApplicationError;
use domain::repositories::{DealRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Get a deal by ID, enforcing visibility.
#[derive(Clone)]
pub struct GetDeal {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl GetDeal {
    pub fn new(deal_repo: Arc<dyn DealRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self), fields(deal_id = %deal_id, user_id = %user_id))]
    pub async fn execute(
        &self,
        deal_id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<DealResult, ApplicationError> {
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        if !is_admin {
            let visible_party_ids: Vec<Uuid> = aggregate
                .participations
                .iter()
                .map(|p| p.party_id)
                .collect();

            let is_member = match party_id {
                Some(pid) => visible_party_ids.contains(&pid),
                None => false,
            };

            if !is_member {
                // Check if user is a member of any participating party.
                let mut member_of_any = false;
                for pid in &visible_party_ids {
                    if self
                        .party_repo
                        .is_user_member_of_party(user_id, *pid)
                        .await?
                    {
                        member_of_any = true;
                        break;
                    }
                }
                if !member_of_any {
                    return Err(ApplicationError::DealNotFound);
                }
            }
        }

        info!(%deal_id, "fetched deal");
        Ok(map_aggregate_to_result(aggregate))
    }
}
