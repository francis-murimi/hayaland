use crate::errors::ApplicationError;
use crate::notifications::LifecycleNotifier;
use domain::entities::{
    DealRole, DealStatus, NotificationType, PlatformWallet, Transaction, TransactionStatus,
    TransactionType,
};
use domain::repositories::{DealRepository, WalletRepository};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Result of a successful deal settlement.
#[derive(Debug, Clone)]
pub struct SettlementResult {
    pub transaction_ids: Vec<Uuid>,
}

/// Orchestrate the final settlement of a deal on completion.
///
/// Uses the deal's value distribution to:
/// - deduct the platform fee from the consumer's escrow,
/// - release the supplier share from escrow to the supplier wallet,
/// - release the enhancer share from escrow to the enhancer wallet.
#[derive(Clone)]
pub struct SettlementSaga {
    deal_repo: Arc<dyn DealRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
    notifier: Option<Arc<LifecycleNotifier>>,
}

impl SettlementSaga {
    pub fn new(deal_repo: Arc<dyn DealRepository>, wallet_repo: Arc<dyn WalletRepository>) -> Self {
        Self {
            deal_repo,
            wallet_repo,
            notifier: None,
        }
    }

    pub fn with_notifier(mut self, notifier: Arc<LifecycleNotifier>) -> Self {
        self.notifier = Some(notifier);
        self
    }

    #[instrument(skip(self), fields(deal_id = %deal_id))]
    pub async fn execute(
        &self,
        deal_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<SettlementResult, ApplicationError> {
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        if aggregate.deal.deal_status != DealStatus::Executing {
            return Err(ApplicationError::InvalidStateTransition {
                from: aggregate.deal.deal_status.as_str().to_string(),
                to: DealStatus::Completed.as_str().to_string(),
            });
        }

        let value_distribution = self
            .deal_repo
            .find_value_distribution_by_deal(deal_id)
            .await?
            .ok_or_else(|| ApplicationError::SettlementFailed {
                reason: "value distribution is required to settle a deal".to_string(),
            })?;

        let participations = aggregate.participations;
        let consumer = self.require_participant(&participations, DealRole::Consumer)?;
        let supplier = self.require_participant(&participations, DealRole::Supplier)?;
        let enhancer = self.require_participant(&participations, DealRole::Enhancer)?;

        self.ensure_wallet(supplier).await?;
        self.ensure_wallet(enhancer).await?;

        let mut consumer_wallet = self.require_wallet(consumer).await?;

        if consumer_wallet.escrow_balance < value_distribution.consumer_cost_amount {
            return Err(ApplicationError::SettlementFailed {
                reason: format!(
                    "consumer escrow balance {} is less than the settlement obligation {}",
                    consumer_wallet.escrow_balance, value_distribution.consumer_cost_amount
                ),
            });
        }

        let mut transaction_ids = Vec::with_capacity(3);

        // 1. Platform fee from consumer escrow.
        if value_distribution.platform_fee_amount > Decimal::ZERO {
            consumer_wallet.deduct_fee_from_escrow(value_distribution.platform_fee_amount)?;
            let fee_txn = Transaction::new(
                Uuid::now_v7(),
                deal_id,
                TransactionType::Fee,
                Some(consumer),
                None,
                value_distribution.platform_fee_amount,
                Some("platform fee on deal completion".to_string()),
                TransactionStatus::Verified,
                None,
                None,
            );
            self.wallet_repo
                .record_transaction(&consumer_wallet, &fee_txn)
                .await?;
            transaction_ids.push(fee_txn.id);
        }

        // 2. Supplier share from consumer escrow to supplier wallet.
        if value_distribution.supplier_share_amount > Decimal::ZERO {
            let mut supplier_wallet = self.require_wallet(supplier).await?;
            consumer_wallet.debit_escrow(value_distribution.supplier_share_amount)?;
            supplier_wallet.credit_balance(value_distribution.supplier_share_amount)?;
            let supplier_txn = Transaction::new(
                Uuid::now_v7(),
                deal_id,
                TransactionType::EscrowRelease,
                Some(consumer),
                Some(supplier),
                value_distribution.supplier_share_amount,
                Some("supplier share on deal completion".to_string()),
                TransactionStatus::Verified,
                None,
                None,
            );
            self.wallet_repo
                .record_multi_party_transaction(
                    &[consumer_wallet.clone(), supplier_wallet],
                    &supplier_txn,
                )
                .await?;
            transaction_ids.push(supplier_txn.id);
        }

        // 3. Enhancer share from consumer escrow to enhancer wallet.
        if value_distribution.enhancer_share_amount > Decimal::ZERO {
            let mut enhancer_wallet = self.require_wallet(enhancer).await?;
            consumer_wallet.debit_escrow(value_distribution.enhancer_share_amount)?;
            enhancer_wallet.credit_balance(value_distribution.enhancer_share_amount)?;
            let enhancer_txn = Transaction::new(
                Uuid::now_v7(),
                deal_id,
                TransactionType::EscrowRelease,
                Some(consumer),
                Some(enhancer),
                value_distribution.enhancer_share_amount,
                Some("enhancer share on deal completion".to_string()),
                TransactionStatus::Verified,
                None,
                None,
            );
            self.wallet_repo
                .record_multi_party_transaction(&[consumer_wallet, enhancer_wallet], &enhancer_txn)
                .await?;
            transaction_ids.push(enhancer_txn.id);
        }

        info!(
            %deal_id,
            transaction_count = transaction_ids.len(),
            "settled deal"
        );

        self.emit_deal_completed_notification(&aggregate.deal, actor_user_id)
            .await;

        Ok(SettlementResult { transaction_ids })
    }

    fn require_participant(
        &self,
        participations: &[domain::entities::DealParticipation],
        role: DealRole,
    ) -> Result<Uuid, ApplicationError> {
        participations
            .iter()
            .find(|p| p.role == role)
            .map(|p| p.party_id)
            .ok_or_else(|| ApplicationError::SettlementFailed {
                reason: format!("deal has no {} participation", role.as_str().to_lowercase()),
            })
    }

    async fn require_wallet(&self, party_id: Uuid) -> Result<PlatformWallet, ApplicationError> {
        self.wallet_repo
            .find_by_party_id(party_id)
            .await?
            .ok_or(ApplicationError::NotFound)
    }

    async fn ensure_wallet(&self, party_id: Uuid) -> Result<(), ApplicationError> {
        if self.wallet_repo.find_by_party_id(party_id).await?.is_none() {
            let wallet = PlatformWallet::new(Uuid::now_v7(), party_id);
            self.wallet_repo.create(&wallet).await?;
        }
        Ok(())
    }

    async fn emit_deal_completed_notification(
        &self,
        deal: &domain::entities::Deal,
        actor_user_id: Uuid,
    ) {
        let Some(notifier) = self.notifier.as_ref() else {
            return;
        };
        let metadata = serde_json::json!({
            "deal_name": deal.deal_title.as_str(),
            "deal_id": deal.id,
        });
        let result = notifier
            .notify_deal_participants(
                actor_user_id,
                deal.id,
                NotificationType::DealCompleted,
                metadata,
            )
            .await;
        notifier.fire_and_forget(result, "deal completed settlement");
    }
}
