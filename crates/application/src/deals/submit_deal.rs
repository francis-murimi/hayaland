use crate::deals::create_deal::map_aggregate_to_result;
use crate::deals::dto::{DealResult, SubmitDealCommand};
use crate::errors::ApplicationError;
use domain::entities::DealStatus;
use domain::repositories::{DealRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Submit a draft deal, moving it to SUGGESTED.
#[derive(Clone)]
pub struct SubmitDeal {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl SubmitDeal {
    pub fn new(deal_repo: Arc<dyn DealRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self), fields(deal_id = %deal_id))]
    pub async fn execute(
        &self,
        deal_id: Uuid,
        cmd: SubmitDealCommand,
    ) -> Result<DealResult, ApplicationError> {
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        let mut deal = aggregate.deal;

        if deal.deal_status != DealStatus::Draft {
            return Err(ApplicationError::Validation(vec![
                "only draft deals can be submitted".to_string(),
            ]));
        }

        if !self
            .party_repo
            .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
            .await?
            || deal.initiator_party_id != cmd.actor_party_id
        {
            return Err(ApplicationError::Forbidden);
        }

        // Ensure all three participations are present.
        if aggregate.participations.len() != 3 {
            return Err(ApplicationError::Validation(vec![
                "deal must have three participations before submission".to_string(),
            ]));
        }

        deal.transition(DealStatus::Suggested)
            .map_err(ApplicationError::from)?;

        self.deal_repo.update(&deal).await?;
        self.deal_repo
            .record_history(deal_id, "DEAL_SUBMITTED", Some(cmd.actor_party_id), None)
            .await?;

        info!(%deal_id, "submitted deal");
        Ok(map_aggregate_to_result(
            domain::repositories::DealAggregate {
                deal,
                participations: aggregate.participations,
            },
        ))
    }
}
