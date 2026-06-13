use crate::deals::dto::{SetValueDistributionCommand, ValueDistributionResult};
use crate::errors::ApplicationError;
use domain::entities::ValueDistribution;
use domain::repositories::{DealRepository, PartyRepository};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Set or replace the value distribution for a deal.
#[derive(Clone)]
pub struct SetValueDistribution {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl SetValueDistribution {
    pub fn new(deal_repo: Arc<dyn DealRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(deal_id = %cmd.deal_id))]
    pub async fn execute(
        &self,
        cmd: SetValueDistributionCommand,
    ) -> Result<ValueDistributionResult, ApplicationError> {
        if !self
            .party_repo
            .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
            .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(cmd.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        if !matches!(
            aggregate.deal.deal_status,
            domain::entities::DealStatus::Draft
                | domain::entities::DealStatus::Suggested
                | domain::entities::DealStatus::PendingReview
                | domain::entities::DealStatus::Negotiating
                | domain::entities::DealStatus::OnHold
                | domain::entities::DealStatus::AwaitingParty
        ) {
            return Err(ApplicationError::Validation(vec![format!(
                "deal is {} and value distribution cannot be changed",
                aggregate.deal.deal_status.as_str()
            )]));
        }

        let is_participant = self
            .deal_repo
            .is_party_participant(cmd.deal_id, cmd.actor_party_id)
            .await?;
        if !is_participant {
            return Err(ApplicationError::Forbidden);
        }

        let mut distribution = ValueDistribution {
            id: Uuid::now_v7(),
            deal_id: cmd.deal_id,
            total_value: cmd.total_value,
            currency: "POINTS".to_string(),
            distribution_model: cmd.distribution_model,
            supplier_share_percentage: cmd.supplier_share_percentage,
            supplier_share_amount: Decimal::ZERO,
            consumer_cost_percentage: cmd.consumer_cost_percentage,
            consumer_cost_amount: Decimal::ZERO,
            enhancer_share_percentage: cmd.enhancer_share_percentage,
            enhancer_share_amount: Decimal::ZERO,
            platform_fee_percentage: cmd.platform_fee_percentage,
            platform_fee_amount: Decimal::ZERO,
            payment_schedule: cmd.payment_schedule,
            win_win_win_score: None,
            created_at: time::OffsetDateTime::now_utc(),
            updated_at: time::OffsetDateTime::now_utc(),
        };

        distribution.recalculate_amounts();
        distribution.validate()?;

        self.deal_repo.set_value_distribution(&distribution).await?;
        self.deal_repo
            .update_value_totals(
                cmd.deal_id,
                distribution.total_value,
                distribution.platform_fee_percentage,
                distribution.platform_fee_amount,
            )
            .await?;
        self.deal_repo
            .record_history(
                cmd.deal_id,
                "VALUE_DISTRIBUTION_SET",
                Some(cmd.actor_party_id),
                None,
            )
            .await?;

        info!(distribution_id = %distribution.id, "set value distribution");
        Ok(map_to_result(distribution))
    }
}

/// Fetch the value distribution for a deal.
#[derive(Clone)]
pub struct GetValueDistribution {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl GetValueDistribution {
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
        user_id: Uuid,
        party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<ValueDistributionResult, ApplicationError> {
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

        let distribution = self
            .deal_repo
            .find_value_distribution_by_deal(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        Ok(map_to_result(distribution))
    }
}

fn map_to_result(vd: ValueDistribution) -> ValueDistributionResult {
    ValueDistributionResult {
        id: vd.id,
        deal_id: vd.deal_id,
        total_value: vd.total_value,
        currency: vd.currency,
        distribution_model: vd.distribution_model,
        supplier_share_percentage: vd.supplier_share_percentage,
        supplier_share_amount: vd.supplier_share_amount,
        consumer_cost_percentage: vd.consumer_cost_percentage,
        consumer_cost_amount: vd.consumer_cost_amount,
        enhancer_share_percentage: vd.enhancer_share_percentage,
        enhancer_share_amount: vd.enhancer_share_amount,
        platform_fee_percentage: vd.platform_fee_percentage,
        platform_fee_amount: vd.platform_fee_amount,
        payment_schedule: vd.payment_schedule,
        win_win_win_score: vd.win_win_win_score,
    }
}
