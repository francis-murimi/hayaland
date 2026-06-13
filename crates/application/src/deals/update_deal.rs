use crate::deals::create_deal::map_deal_to_result;
use crate::deals::dto::{DealParticipationResult, DealResult, UpdateDealCommand};
use crate::errors::ApplicationError;
use domain::entities::DealStatus;
use domain::repositories::{DealRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Update a draft deal.
#[derive(Clone)]
pub struct UpdateDeal {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl UpdateDeal {
    pub fn new(deal_repo: Arc<dyn DealRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(deal_id = %deal_id))]
    pub async fn execute(
        &self,
        deal_id: Uuid,
        cmd: UpdateDealCommand,
    ) -> Result<DealResult, ApplicationError> {
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        let mut deal = aggregate.deal;
        let participations = aggregate.participations;

        // Only draft deals can be edited via this use case.
        if deal.deal_status != DealStatus::Draft {
            return Err(ApplicationError::Validation(vec![
                "only draft deals can be updated".to_string(),
            ]));
        }

        // Only initiator party members can edit.
        if !self
            .party_repo
            .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
            .await?
            || deal.initiator_party_id != cmd.actor_party_id
        {
            return Err(ApplicationError::Forbidden);
        }

        if let Some(title) = cmd.title {
            deal.deal_title =
                domain::entities::DealTitle::new(&title).map_err(ApplicationError::from)?;
        }
        if cmd.description.is_some() {
            deal.deal_description = cmd.description;
        }
        if let Some(domain_category_id) = cmd.domain_category_id {
            deal.domain_category_id = domain_category_id;
        }
        deal.set_timeline(cmd.expected_start_date, cmd.expected_end_date, cmd.timeline);
        if let (Some(lat), Some(lng)) = (cmd.latitude, cmd.longitude) {
            deal.location =
                Some(domain::entities::GeoPoint::new(lat, lng).map_err(ApplicationError::from)?);
        }

        self.deal_repo.update(&deal).await?;
        self.deal_repo
            .record_history(deal_id, "DEAL_UPDATED", Some(cmd.actor_party_id), None)
            .await?;

        info!(%deal_id, "updated draft deal");
        Ok(map_deal_to_result(
            deal,
            participations
                .into_iter()
                .map(|p| DealParticipationResult {
                    id: p.id,
                    party_id: p.party_id,
                    role: p.role,
                    participation_status: p.participation_status.as_str().to_string(),
                    is_initiator: p.is_initiator,
                    value_share_percentage: p.value_share_percentage,
                    value_share_amount: p.value_share_amount,
                    invited_at: p.invited_at,
                    responded_at: p.responded_at,
                })
                .collect(),
        ))
    }
}
