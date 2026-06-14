use crate::errors::ApplicationError;
use crate::payments::deposit_points::validate_amount;
use crate::payments::dto::{AdjustmentDirection, RecordAdjustmentCommand, TransactionResult};
use domain::entities::{Transaction, TransactionStatus, TransactionType};
use domain::repositories::{DealRepository, PartyRepository, WalletRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Record an administrative adjustment for a deal.
#[derive(Clone)]
pub struct RecordAdjustment {
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl RecordAdjustment {
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
        cmd: RecordAdjustmentCommand,
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

        let (from_party_id, to_party_id) = match cmd.direction {
            AdjustmentDirection::Credit => {
                wallet.deposit(cmd.amount)?;
                (None, Some(cmd.actor_party_id))
            }
            AdjustmentDirection::Debit => {
                wallet.withdraw(cmd.amount)?;
                (Some(cmd.actor_party_id), None)
            }
        };

        let transaction = Transaction::new(
            Uuid::now_v7(),
            cmd.deal_id,
            TransactionType::Adjustment,
            from_party_id,
            to_party_id,
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
            direction = ?cmd.direction,
            "recorded adjustment"
        );

        Ok(transaction.into())
    }
}
