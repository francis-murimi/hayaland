use crate::deals::dto::{CreateDealCommand, DealParticipationResult, DealResult};
use crate::errors::ApplicationError;
use domain::entities::{Deal, DealParticipation, DealRole, DealTitle};
use domain::repositories::{DealRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Create a new draft deal.
#[derive(Clone)]
pub struct CreateDeal {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl CreateDeal {
    pub fn new(deal_repo: Arc<dyn DealRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(title = %cmd.title))]
    pub async fn execute(&self, cmd: CreateDealCommand) -> Result<DealResult, ApplicationError> {
        let title = DealTitle::new(&cmd.title).map_err(ApplicationError::from)?;

        // Verify actor party exists and is active.
        let actor_party = self
            .party_repo
            .find_by_id(cmd.actor_party_id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;
        if !actor_party.is_active {
            return Err(ApplicationError::PartyNotFound);
        }

        // Verify actor is a member of the actor party.
        if !self
            .party_repo
            .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
            .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        // Determine initiator role from actor party's roles.
        let roles = self
            .party_repo
            .list_roles(cmd.actor_party_id)
            .await?
            .into_iter()
            .map(|(role, _profile)| role)
            .collect::<Vec<_>>();
        if roles.is_empty() {
            return Err(ApplicationError::Validation(vec![
                "actor party has no deal roles".to_string(),
            ]));
        }
        // Prefer supplier, then consumer, then enhancer.
        let initiator_role = if roles.contains(&DealRole::Supplier) {
            DealRole::Supplier
        } else if roles.contains(&DealRole::Consumer) {
            DealRole::Consumer
        } else {
            DealRole::Enhancer
        };

        // Validate other parties exist and are active, and have the required role.
        let supplier_party_id = match initiator_role {
            DealRole::Supplier => cmd.actor_party_id,
            _ => {
                self.resolve_participant(cmd.consumer_party_id, DealRole::Supplier)
                    .await?
            }
        };
        let consumer_party_id = match initiator_role {
            DealRole::Consumer => cmd.actor_party_id,
            _ => {
                self.resolve_participant(cmd.consumer_party_id, DealRole::Consumer)
                    .await?
            }
        };
        let enhancer_party_id = match initiator_role {
            DealRole::Enhancer => cmd.actor_party_id,
            _ => {
                self.resolve_participant(cmd.enhancer_party_id, DealRole::Enhancer)
                    .await?
            }
        };

        // All three parties must be distinct.
        let mut ids = [supplier_party_id, consumer_party_id, enhancer_party_id];
        ids.sort();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(ApplicationError::Validation(vec![
                "deal must involve three distinct parties".to_string(),
            ]));
        }

        let deal_id = Uuid::now_v7();
        let deal_reference = self.deal_repo.next_deal_reference().await?;
        let mut deal = Deal::new(
            deal_id,
            deal_reference,
            title,
            cmd.domain_category_id,
            cmd.actor_party_id,
            initiator_role,
        );
        deal.deal_description = cmd.description;
        deal.set_timeline(cmd.expected_start_date, cmd.expected_end_date, cmd.timeline);
        if let (Some(lat), Some(lng)) = (cmd.latitude, cmd.longitude) {
            deal.location =
                Some(domain::entities::GeoPoint::new(lat, lng).map_err(ApplicationError::from)?);
        }

        let participations = vec![
            DealParticipation::new(
                Uuid::now_v7(),
                deal_id,
                supplier_party_id,
                DealRole::Supplier,
                supplier_party_id == cmd.actor_party_id,
            ),
            DealParticipation::new(
                Uuid::now_v7(),
                deal_id,
                consumer_party_id,
                DealRole::Consumer,
                consumer_party_id == cmd.actor_party_id,
            ),
            DealParticipation::new(
                Uuid::now_v7(),
                deal_id,
                enhancer_party_id,
                DealRole::Enhancer,
                enhancer_party_id == cmd.actor_party_id,
            ),
        ];

        let aggregate = domain::repositories::DealAggregate {
            deal,
            participations,
        };

        self.deal_repo.create(&aggregate).await?;
        self.deal_repo
            .record_history(deal_id, "DEAL_CREATED", Some(cmd.actor_party_id), None)
            .await?;

        info!(%deal_id, actor = %cmd.actor_user_id, "created draft deal");
        Ok(map_aggregate_to_result(aggregate))
    }

    async fn resolve_participant(
        &self,
        party_id: Uuid,
        required_role: DealRole,
    ) -> Result<Uuid, ApplicationError> {
        let party = self
            .party_repo
            .find_by_id(party_id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;
        if !party.is_active {
            return Err(ApplicationError::PartyNotFound);
        }
        let roles = self.party_repo.list_roles(party_id).await?;
        if !roles.iter().any(|(role, _profile)| *role == required_role) {
            return Err(ApplicationError::Validation(vec![format!(
                "party {party_id} does not have the {required_role:?} role"
            )]));
        }
        Ok(party_id)
    }
}

pub(crate) fn map_aggregate_to_result(
    aggregate: domain::repositories::DealAggregate,
) -> DealResult {
    map_deal_to_result(
        aggregate.deal,
        aggregate
            .participations
            .into_iter()
            .map(map_participation_to_result)
            .collect(),
    )
}

pub(crate) fn map_deal_to_result(
    deal: Deal,
    participations: Vec<DealParticipationResult>,
) -> DealResult {
    DealResult {
        id: deal.id,
        deal_reference: deal.deal_reference,
        title: deal.deal_title.as_str().to_owned(),
        description: deal.deal_description,
        domain_category_id: deal.domain_category_id,
        initiator_party_id: deal.initiator_party_id,
        initiator_role: deal.initiator_role,
        deal_status: deal.deal_status,
        expected_start_date: deal.expected_start_date,
        expected_end_date: deal.expected_end_date,
        actual_start_date: deal.actual_start_date,
        actual_end_date: deal.actual_end_date,
        timeline: deal.timeline,
        latitude: deal.location.map(|l| l.latitude),
        longitude: deal.location.map(|l| l.longitude),
        total_deal_value: deal.total_deal_value,
        currency: deal.currency,
        platform_fee_percentage: deal.platform_fee_percentage,
        platform_fee_amount: deal.platform_fee_amount,
        win_win_win_validated: deal.win_win_win_validated,
        validation_score: deal.validation_score,
        is_public: deal.is_public,
        current_state_entered_at: deal.current_state_entered_at,
        created_at: deal.created_at,
        updated_at: deal.updated_at,
        participations,
    }
}

pub(crate) fn map_participation_to_result(
    participation: DealParticipation,
) -> DealParticipationResult {
    DealParticipationResult {
        id: participation.id,
        party_id: participation.party_id,
        role: participation.role,
        participation_status: participation.participation_status.as_str().to_string(),
        is_initiator: participation.is_initiator,
        value_share_percentage: participation.value_share_percentage,
        value_share_amount: participation.value_share_amount,
        invited_at: participation.invited_at,
        responded_at: participation.responded_at,
    }
}
