use crate::entities::{DealWallet, PlatformWallet, Transaction, TransactionApproval};
use crate::errors::DomainError;
use async_trait::async_trait;
use uuid::Uuid;

/// Filters for listing a party's transactions.
#[derive(Debug, Clone)]
pub struct TransactionFilters {
    pub deal_id: Option<Uuid>,
    pub status: Option<String>,
    pub transaction_type: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for TransactionFilters {
    fn default() -> Self {
        Self {
            deal_id: None,
            status: None,
            transaction_type: None,
            limit: 100,
            offset: 0,
        }
    }
}

/// Outbound port for wallet persistence and per-deal ledger queries.
#[async_trait]
pub trait WalletRepository: Send + Sync {
    /// Create a new wallet container for a party.
    async fn create(&self, wallet: &PlatformWallet) -> Result<(), DomainError>;

    /// Find a party's wallet container by party id.
    async fn find_by_party_id(&self, party_id: Uuid)
        -> Result<Option<PlatformWallet>, DomainError>;

    /// Update the aggregate balances of a wallet container.
    async fn update(&self, wallet: &PlatformWallet) -> Result<(), DomainError>;

    /// Persist a transaction and update the wallet container atomically.
    async fn record_transaction(
        &self,
        wallet: &PlatformWallet,
        transaction: &Transaction,
    ) -> Result<(), DomainError>;

    /// Find transactions for a party (as source or beneficiary) with optional filters.
    async fn find_transactions(
        &self,
        party_id: Uuid,
        filters: &TransactionFilters,
    ) -> Result<Vec<Transaction>, DomainError>;

    /// Count transactions for a party matching the filters.
    async fn count_transactions(
        &self,
        party_id: Uuid,
        filters: &TransactionFilters,
    ) -> Result<i64, DomainError>;

    /// Compute the per-deal sub-wallet for a party.
    async fn compute_deal_wallet(
        &self,
        party_id: Uuid,
        deal_id: Uuid,
    ) -> Result<Option<DealWallet>, DomainError>;

    /// Persist a pending transaction without mutating wallet balances.
    async fn record_pending_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<(), DomainError>;

    /// Find a transaction by ID.
    async fn find_transaction_by_id(&self, id: Uuid) -> Result<Option<Transaction>, DomainError>;

    /// Find all approvals recorded for a transaction.
    async fn find_approvals_for_transaction(
        &self,
        transaction_id: Uuid,
    ) -> Result<Vec<TransactionApproval>, DomainError>;

    /// Record one approval and, if it finalises the transaction, apply the
    /// ledger mutation atomically.
    async fn record_approval_and_finalise(
        &self,
        transaction: &Transaction,
        approval: &TransactionApproval,
        wallet_mutations: &[(Uuid, PlatformWallet)],
    ) -> Result<(), DomainError>;

    /// List pending transactions where the given party is a required approver
    /// and has not yet voted.
    async fn find_pending_transactions_for_party(
        &self,
        party_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Transaction>, DomainError>;

    /// Count pending transactions where the given party is a required approver
    /// and has not yet voted.
    async fn count_pending_transactions_for_party(
        &self,
        party_id: Uuid,
    ) -> Result<i64, DomainError>;
}
