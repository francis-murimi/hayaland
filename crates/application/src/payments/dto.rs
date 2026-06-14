use domain::entities::{ApprovalDecision, Currency, PlatformWallet, Transaction};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Command to record a deposit into the party's wallet container.
#[derive(Debug, Clone, Deserialize)]
pub struct DepositPointsCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub amount: Decimal,
    pub description: Option<String>,
    pub payment_method: Option<String>,
    pub external_reference: Option<String>,
}

/// Command to record a withdrawal from the party's wallet container.
#[derive(Debug, Clone, Deserialize)]
pub struct WithdrawPointsCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub amount: Decimal,
    pub description: Option<String>,
    pub payment_method: Option<String>,
    pub external_reference: Option<String>,
}

/// Command to hold funds in escrow for a deal.
#[derive(Debug, Clone, Deserialize)]
pub struct HoldEscrowCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub amount: Decimal,
    pub description: Option<String>,
    pub payment_method: Option<String>,
    pub external_reference: Option<String>,
}

/// Command to release escrow funds back to available balance for a deal.
#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseEscrowCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub amount: Decimal,
    pub description: Option<String>,
    pub payment_method: Option<String>,
    pub external_reference: Option<String>,
}

/// Source of funds for a fee deduction.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeeSource {
    Balance,
    Escrow,
}

/// Command to deduct a fee from a party's wallet for a deal.
#[derive(Debug, Clone, Deserialize)]
pub struct DeductFeeCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub amount: Decimal,
    pub source: FeeSource,
    pub description: Option<String>,
    pub payment_method: Option<String>,
    pub external_reference: Option<String>,
}

/// Direction of an adjustment entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdjustmentDirection {
    Credit,
    Debit,
}

/// Command to record an administrative adjustment for a deal.
#[derive(Debug, Clone, Deserialize)]
pub struct RecordAdjustmentCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub amount: Decimal,
    pub direction: AdjustmentDirection,
    pub description: Option<String>,
    pub payment_method: Option<String>,
    pub external_reference: Option<String>,
}

/// Filters for listing wallet transactions.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListTransactionsQuery {
    pub deal_id: Option<Uuid>,
    pub status: Option<String>,
    pub transaction_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Full wallet container representation returned by use cases.
#[derive(Debug, Clone, Serialize)]
pub struct WalletResult {
    pub id: Uuid,
    pub party_id: Uuid,
    pub balance: Decimal,
    pub escrow_balance: Decimal,
    pub pending_balance: Decimal,
    pub total_deposited: Decimal,
    pub total_withdrawn: Decimal,
    pub currency: Currency,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<PlatformWallet> for WalletResult {
    fn from(wallet: PlatformWallet) -> Self {
        Self {
            id: wallet.id,
            party_id: wallet.party_id,
            balance: wallet.balance,
            escrow_balance: wallet.escrow_balance,
            pending_balance: wallet.pending_balance,
            total_deposited: wallet.total_deposited,
            total_withdrawn: wallet.total_withdrawn,
            currency: wallet.currency,
            is_active: wallet.is_active,
            created_at: wallet.created_at,
            updated_at: wallet.updated_at,
        }
    }
}

/// Per-deal sub-wallet returned by use cases.
#[derive(Debug, Clone, Serialize)]
pub struct DealWalletResult {
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
    pub currency: Currency,
}

/// Single transaction returned by use cases.
#[derive(Debug, Clone, Serialize)]
pub struct TransactionResult {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub transaction_type: String,
    pub amount: Decimal,
    pub currency: Currency,
    pub status: String,
    pub description: Option<String>,
    pub created_at: OffsetDateTime,
}

impl From<Transaction> for TransactionResult {
    fn from(txn: Transaction) -> Self {
        Self {
            id: txn.id,
            deal_id: txn.deal_id,
            transaction_type: txn.transaction_type.as_str().to_string(),
            amount: txn.amount,
            currency: txn.currency,
            status: txn.status.as_str().to_string(),
            description: txn.description,
            created_at: txn.created_at,
        }
    }
}

/// Paginated list of transactions.
#[derive(Debug, Clone, Serialize)]
pub struct ListTransactionsResult {
    pub transactions: Vec<TransactionResult>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Command to approve or reject a pending transaction.
#[derive(Debug, Clone, Deserialize)]
pub struct ApproveTransactionCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub transaction_id: Uuid,
    pub decision: ApprovalDecision,
    pub comment: Option<String>,
}

/// Query for pending transactions awaiting the actor's party approval.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListPendingApprovalsQuery {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Approval recorded against a transaction.
#[derive(Debug, Clone, Serialize)]
pub struct TransactionApprovalResult {
    pub id: Uuid,
    pub party_id: Uuid,
    pub approved_by_user_id: Uuid,
    pub decision: String,
    pub comment: Option<String>,
    pub created_at: OffsetDateTime,
}

impl From<domain::entities::TransactionApproval> for TransactionApprovalResult {
    fn from(a: domain::entities::TransactionApproval) -> Self {
        Self {
            id: a.id,
            party_id: a.party_id,
            approved_by_user_id: a.approved_by_user_id,
            decision: a.decision.as_str().to_string(),
            comment: a.comment,
            created_at: a.created_at,
        }
    }
}

/// Transaction with its current approvals.
#[derive(Debug, Clone, Serialize)]
pub struct TransactionWithApprovalsResult {
    #[serde(flatten)]
    pub transaction: TransactionResult,
    pub approvals: Vec<TransactionApprovalResult>,
    pub approvals_required: i32,
    pub approvals_received: i32,
}

/// Paginated list of pending transactions awaiting approval.
#[derive(Debug, Clone, Serialize)]
pub struct ListPendingApprovalsResult {
    pub transactions: Vec<TransactionResult>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
