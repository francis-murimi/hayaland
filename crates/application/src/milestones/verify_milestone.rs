use crate::errors::ApplicationError;
use crate::milestones::access::{allow_milestone_mutations, ensure_participant};
use crate::milestones::dto::{MilestoneActionCommand, MilestoneWithTransactionResult};
use domain::entities::{Milestone, PlatformWallet, Transaction, TransactionType};
use domain::repositories::{
    DealRepository, MilestoneRepository, PartyRepository, WalletRepository,
};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

#[derive(Clone)]
pub struct VerifyMilestone {
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    milestone_repo: Arc<dyn MilestoneRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl VerifyMilestone {
    pub fn new(
        party_repo: Arc<dyn PartyRepository>,
        deal_repo: Arc<dyn DealRepository>,
        milestone_repo: Arc<dyn MilestoneRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
    ) -> Self {
        Self {
            party_repo,
            deal_repo,
            milestone_repo,
            wallet_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(milestone_id = %cmd.milestone_id))]
    pub async fn execute(
        &self,
        cmd: MilestoneActionCommand,
    ) -> Result<MilestoneWithTransactionResult, ApplicationError> {
        let milestone = self
            .milestone_repo
            .find_by_id(cmd.milestone_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        ensure_participant(
            &self.party_repo,
            &self.deal_repo,
            cmd.actor_user_id,
            cmd.actor_party_id,
            milestone.deal_id,
        )
        .await?;

        let deal = self
            .deal_repo
            .find_by_id(milestone.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;
        allow_milestone_mutations(deal.deal_status)?;

        let mut milestone = milestone;
        milestone.verify(cmd.actor_party_id)?;

        let triggered_transaction_id = if let Some(amount) = milestone.payment_trigger_amount {
            Some(self.create_pending_release(&milestone, amount).await?)
        } else {
            None
        };

        self.milestone_repo.update(&milestone).await?;

        info!(
            milestone_id = %milestone.id,
            transaction_id = ?triggered_transaction_id,
            "verified milestone"
        );

        Ok(MilestoneWithTransactionResult {
            milestone: milestone.into(),
            triggered_transaction_id,
        })
    }

    async fn create_pending_release(
        &self,
        milestone: &Milestone,
        amount: Decimal,
    ) -> Result<Uuid, ApplicationError> {
        let participations = self
            .deal_repo
            .find_participations_by_deal(milestone.deal_id)
            .await?;

        let consumer = participations
            .iter()
            .find(|p| p.role == domain::entities::DealRole::Consumer)
            .map(|p| p.party_id)
            .ok_or_else(|| {
                ApplicationError::Validation(vec![
                    "deal has no consumer party to fund the release".to_string()
                ])
            })?;

        let involved_party_ids: Vec<Uuid> = participations.iter().map(|p| p.party_id).collect();

        let consumer_wallet = self
            .wallet_repo
            .find_by_party_id(consumer)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        if consumer_wallet.escrow_balance < amount {
            return Err(ApplicationError::Validation(vec![
                "consumer escrow balance is insufficient for milestone release".to_string(),
            ]));
        }

        self.ensure_wallet_exists(milestone.assigned_to_party_id)
            .await?;

        let txn = Transaction::new_pending(
            Uuid::now_v7(),
            milestone.deal_id,
            TransactionType::EscrowRelease,
            Some(consumer),
            Some(milestone.assigned_to_party_id),
            amount,
            involved_party_ids.len() as i32,
            involved_party_ids,
            milestone
                .description
                .clone()
                .or_else(|| Some("milestone release".to_string())),
            None,
            None,
        );

        let txn_id = txn.id;
        self.wallet_repo.record_pending_transaction(&txn).await?;
        Ok(txn_id)
    }

    async fn ensure_wallet_exists(&self, party_id: Uuid) -> Result<(), ApplicationError> {
        if self.wallet_repo.find_by_party_id(party_id).await?.is_none() {
            let wallet = PlatformWallet::new(Uuid::now_v7(), party_id);
            self.wallet_repo.create(&wallet).await?;
        }
        Ok(())
    }
}
