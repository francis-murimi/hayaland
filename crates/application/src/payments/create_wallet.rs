use domain::entities::PlatformWallet;
use domain::repositories::WalletRepository;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Create a wallet container for a party.
///
/// This use case is intended to be invoked automatically when a party is
/// created; it is not exposed directly through the public API.
#[derive(Clone)]
pub struct CreateWallet {
    repo: Arc<dyn WalletRepository>,
}

impl CreateWallet {
    pub fn new(repo: Arc<dyn WalletRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self), fields(party_id = %party_id))]
    pub async fn execute(&self, party_id: Uuid) -> Result<(), domain::errors::DomainError> {
        let wallet = PlatformWallet::new(Uuid::now_v7(), party_id);
        self.repo.create(&wallet).await?;
        info!(%party_id, "created wallet container");
        Ok(())
    }
}
