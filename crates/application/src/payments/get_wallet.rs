use crate::errors::ApplicationError;
use crate::payments::dto::WalletResult;
use domain::repositories::{PartyRepository, WalletRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Read a party's wallet container.
#[derive(Clone)]
pub struct GetWallet {
    party_repo: Arc<dyn PartyRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl GetWallet {
    pub fn new(
        party_repo: Arc<dyn PartyRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
    ) -> Self {
        Self {
            party_repo,
            wallet_repo,
        }
    }

    #[instrument(skip(self), fields(party_id = %party_id))]
    pub async fn execute(
        &self,
        actor_user_id: Uuid,
        party_id: Uuid,
        is_admin: bool,
    ) -> Result<WalletResult, ApplicationError> {
        if !is_admin
            && !self
                .party_repo
                .is_user_member_of_party(actor_user_id, party_id)
                .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        let wallet = self
            .wallet_repo
            .find_by_party_id(party_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        info!(%party_id, "read wallet container");
        Ok(wallet.into())
    }
}
