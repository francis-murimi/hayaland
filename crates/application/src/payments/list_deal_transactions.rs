use crate::errors::ApplicationError;
use crate::payments::dto::{ListTransactionsQuery, ListTransactionsResult};
use domain::repositories::{DealRepository, PartyRepository, TransactionFilters, WalletRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 100;

/// List transactions for a party within a single deal.
#[derive(Clone)]
pub struct ListDealTransactions {
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl ListDealTransactions {
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

    #[instrument(skip(self, query), fields(party_id = %party_id, deal_id = %deal_id))]
    pub async fn execute(
        &self,
        actor_user_id: Uuid,
        party_id: Uuid,
        deal_id: Uuid,
        query: ListTransactionsQuery,
    ) -> Result<ListTransactionsResult, ApplicationError> {
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

        let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let offset = query.offset.unwrap_or(0).max(0);

        let filters = TransactionFilters {
            deal_id: Some(deal_id),
            status: query.status,
            transaction_type: query.transaction_type,
            limit,
            offset,
        };

        let transactions = self
            .wallet_repo
            .find_transactions(party_id, &filters)
            .await?;
        let total = self
            .wallet_repo
            .count_transactions(party_id, &filters)
            .await?;

        info!(%party_id, %deal_id, total, "listed deal transactions");
        Ok(ListTransactionsResult {
            transactions: transactions.into_iter().map(Into::into).collect(),
            total,
            limit,
            offset,
        })
    }
}
