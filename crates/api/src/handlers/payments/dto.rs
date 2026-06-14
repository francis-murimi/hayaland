use application::payments::dto::{
    DealWalletResult, ListTransactionsResult, TransactionResult, WalletResult,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, validator::Validate)]
pub struct DepositRequest {
    #[serde(rename = "dealId")]
    pub deal_id: Uuid,
    pub amount: Decimal,
    pub description: Option<String>,
    pub payment_method: Option<String>,
    pub external_reference: Option<String>,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct WithdrawalRequest {
    #[serde(rename = "dealId")]
    pub deal_id: Uuid,
    pub amount: Decimal,
    pub description: Option<String>,
    pub payment_method: Option<String>,
    pub external_reference: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListTransactionsQuery {
    #[serde(rename = "dealId")]
    pub deal_id: Option<Uuid>,
    pub status: Option<String>,
    #[serde(rename = "transactionType")]
    pub transaction_type: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl From<ListTransactionsQuery> for application::payments::dto::ListTransactionsQuery {
    fn from(query: ListTransactionsQuery) -> Self {
        const DEFAULT_PER_PAGE: i64 = 20;
        const MAX_PER_PAGE: i64 = 100;

        let per_page = query
            .per_page
            .unwrap_or(DEFAULT_PER_PAGE)
            .clamp(1, MAX_PER_PAGE);
        let page = query.page.unwrap_or(1).max(1);
        let offset = (page - 1) * per_page;

        Self {
            deal_id: query.deal_id,
            status: query.status,
            transaction_type: query.transaction_type,
            limit: Some(per_page),
            offset: Some(offset),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletResponse {
    pub wallet_id: Uuid,
    pub party_id: Uuid,
    pub balance: Decimal,
    pub escrow_balance: Decimal,
    pub pending_balance: Decimal,
    pub total_deposited: Decimal,
    pub total_withdrawn: Decimal,
    pub currency: String,
    pub is_active: bool,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

impl From<WalletResult> for WalletResponse {
    fn from(result: WalletResult) -> Self {
        Self {
            wallet_id: result.id,
            party_id: result.party_id,
            balance: result.balance,
            escrow_balance: result.escrow_balance,
            pending_balance: result.pending_balance,
            total_deposited: result.total_deposited,
            total_withdrawn: result.total_withdrawn,
            currency: result.currency.as_str().to_string(),
            is_active: result.is_active,
            created_at: result.created_at,
            updated_at: result.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DealWalletResponse {
    pub deal_id: Uuid,
    pub party_id: Uuid,
    pub deposited: Decimal,
    pub withdrawn: Decimal,
    pub contributed: Decimal,
    pub held_in_escrow: Decimal,
    pub released: Decimal,
    pub fees_paid: Decimal,
    pub pending: Decimal,
    pub net_position: Decimal,
    pub currency: String,
}

impl From<DealWalletResult> for DealWalletResponse {
    fn from(result: DealWalletResult) -> Self {
        Self {
            deal_id: result.deal_id,
            party_id: result.party_id,
            deposited: result.deposited,
            withdrawn: result.withdrawn,
            contributed: result.contributed,
            held_in_escrow: result.held_in_escrow,
            released: result.released,
            fees_paid: result.fees_paid,
            pending: result.pending,
            net_position: result.net_position,
            currency: result.currency.as_str().to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResponse {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub transaction_type: String,
    pub amount: Decimal,
    pub currency: String,
    pub status: String,
    pub description: Option<String>,
    pub created_at: time::OffsetDateTime,
}

impl From<TransactionResult> for TransactionResponse {
    fn from(result: TransactionResult) -> Self {
        Self {
            id: result.id,
            deal_id: result.deal_id,
            transaction_type: result.transaction_type,
            amount: result.amount,
            currency: result.currency.as_str().to_string(),
            status: result.status,
            description: result.description,
            created_at: result.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionsResponse {
    pub transactions: Vec<TransactionResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

impl From<ListTransactionsResult> for TransactionsResponse {
    fn from(result: ListTransactionsResult) -> Self {
        Self {
            transactions: result.transactions.into_iter().map(Into::into).collect(),
            total: result.total,
            page: (result.offset / result.limit) + 1,
            per_page: result.limit,
        }
    }
}
