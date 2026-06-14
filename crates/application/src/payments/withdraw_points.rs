use crate::errors::ApplicationError;
use crate::payments::deposit_points::validate_amount;
use crate::payments::dto::{TransactionResult, WithdrawPointsCommand};
use domain::entities::{Transaction, TransactionStatus, TransactionType};
use domain::repositories::{DealRepository, PartyRepository, WalletRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Record a withdrawal from a party's wallet container for a deal.
#[derive(Clone)]
pub struct WithdrawPoints {
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl WithdrawPoints {
    pub fn new(
        party_repo: Arc<dyn PartyRepository>,
        deal_repo: Arc<dyn DealRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
    ) -> Self {
        Self {
            party_repo,
            deal_repo,
            wallet_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(party_id = %cmd.actor_party_id, deal_id = %cmd.deal_id))]
    pub async fn execute(
        &self,
        cmd: WithdrawPointsCommand,
    ) -> Result<TransactionResult, ApplicationError> {
        validate_amount(cmd.amount)?;

        if !self
            .party_repo
            .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
            .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        if !self
            .deal_repo
            .is_party_participant(cmd.deal_id, cmd.actor_party_id)
            .await?
        {
            return Err(ApplicationError::DealAccessDenied);
        }

        let mut wallet = self
            .wallet_repo
            .find_by_party_id(cmd.actor_party_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        wallet.withdraw(cmd.amount)?;

        let transaction = Transaction::new(
            Uuid::now_v7(),
            cmd.deal_id,
            TransactionType::Withdrawal,
            Some(cmd.actor_party_id),
            None,
            cmd.amount,
            cmd.description,
            TransactionStatus::Verified,
            cmd.payment_method,
            cmd.external_reference,
        );

        self.wallet_repo
            .record_transaction(&wallet, &transaction)
            .await?;

        info!(
            transaction_id = %transaction.id,
            party_id = %cmd.actor_party_id,
            amount = %cmd.amount,
            "recorded withdrawal"
        );

        Ok(transaction.into())
    }
}
