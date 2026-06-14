use crate::errors::ApplicationError;
use crate::payments::dto::DealWalletResult;
use domain::entities::Currency;
use domain::repositories::{DealRepository, PartyRepository, WalletRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Compute a party's per-deal sub-wallet.
#[derive(Clone)]
pub struct GetDealWallet {
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl GetDealWallet {
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

    #[instrument(skip(self), fields(party_id = %party_id, deal_id = %deal_id))]
    pub async fn execute(
        &self,
        actor_user_id: Uuid,
        party_id: Uuid,
        deal_id: Uuid,
        is_admin: bool,
    ) -> Result<DealWalletResult, ApplicationError> {
        if !is_admin {
            if !self
                .party_repo
                .is_user_member_of_party(actor_user_id, party_id)
                .await?
            {
                return Err(ApplicationError::Forbidden);
            }

            if !self
                .deal_repo
                .is_party_participant(deal_id, party_id)
                .await?
            {
                return Err(ApplicationError::DealAccessDenied);
            }
        }

        let deal_wallet = self
            .wallet_repo
            .compute_deal_wallet(party_id, deal_id)
            .await?
            .unwrap_or_else(|| {
                domain::entities::DealWallet::new(party_id, deal_id, Currency::Points)
            });

        info!(%party_id, %deal_id, "computed per-deal sub-wallet");
        Ok(DealWalletResult {
            deal_id: deal_wallet.deal_id,
            party_id: deal_wallet.party_id,
            deposited: deal_wallet.deposited,
            withdrawn: deal_wallet.withdrawn,
            contributed: deal_wallet.contributed,
            held_in_escrow: deal_wallet.held_in_escrow,
            released: deal_wallet.released,
            fees_paid: deal_wallet.fees_paid,
            pending: deal_wallet.pending,
            net_position: deal_wallet.net_position,
            currency: deal_wallet.currency,
        })
    }
}
