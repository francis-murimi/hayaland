use crate::errors::DomainError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Type of point movement recorded in the ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    EscrowHold,
    EscrowRelease,
    Fee,
    Adjustment,
}

impl TransactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionType::Deposit => "DEPOSIT",
            TransactionType::Withdrawal => "WITHDRAWAL",
            TransactionType::EscrowHold => "ESCROW_HOLD",
            TransactionType::EscrowRelease => "ESCROW_RELEASE",
            TransactionType::Fee => "FEE",
            TransactionType::Adjustment => "ADJUSTMENT",
        }
    }
}

impl TryFrom<&str> for TransactionType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "DEPOSIT" => Ok(TransactionType::Deposit),
            "WITHDRAWAL" => Ok(TransactionType::Withdrawal),
            "ESCROW_HOLD" => Ok(TransactionType::EscrowHold),
            "ESCROW_RELEASE" => Ok(TransactionType::EscrowRelease),
            "FEE" => Ok(TransactionType::Fee),
            "ADJUSTMENT" => Ok(TransactionType::Adjustment),
            _ => Err(DomainError::Validation(vec![format!(
                "unknown transaction type: {value}"
            )])),
        }
    }
}

/// Lifecycle status of a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionStatus {
    Pending,
    Verified,
    Complete,
    Rejected,
}

impl TransactionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionStatus::Pending => "PENDING",
            TransactionStatus::Verified => "VERIFIED",
            TransactionStatus::Complete => "COMPLETE",
            TransactionStatus::Rejected => "REJECTED",
        }
    }
}

impl TryFrom<&str> for TransactionStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "PENDING" => Ok(TransactionStatus::Pending),
            "VERIFIED" => Ok(TransactionStatus::Verified),
            "COMPLETE" => Ok(TransactionStatus::Complete),
            "REJECTED" => Ok(TransactionStatus::Rejected),
            _ => Err(DomainError::Validation(vec![format!(
                "unknown transaction status: {value}"
            )])),
        }
    }
}

/// A single point movement in the platform ledger.
///
/// Transactions are always tied to a `deal_id` so that every party has a
/// per-deal sub-wallet view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub milestone_id: Option<Uuid>,
    pub transaction_type: TransactionType,
    pub from_party_id: Option<Uuid>,
    pub to_party_id: Option<Uuid>,
    pub amount: Decimal,
    pub currency: super::Currency,
    pub description: Option<String>,
    pub status: TransactionStatus,
    pub payment_method: Option<String>,
    pub external_reference: Option<String>,
    pub requires_approval: bool,
    pub approvals_required: i32,
    pub approvals_received: i32,
    pub executed_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

impl Transaction {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        deal_id: Uuid,
        transaction_type: TransactionType,
        from_party_id: Option<Uuid>,
        to_party_id: Option<Uuid>,
        amount: Decimal,
        description: Option<String>,
        status: TransactionStatus,
        payment_method: Option<String>,
        external_reference: Option<String>,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            deal_id,
            agreement_id: None,
            milestone_id: None,
            transaction_type,
            from_party_id,
            to_party_id,
            amount,
            currency: super::Currency::Points,
            description,
            status,
            payment_method,
            external_reference,
            requires_approval: false,
            approvals_required: 0,
            approvals_received: 0,
            executed_at: Some(now),
            created_at: now,
        }
    }

    /// Convenience constructor for a simple, verified deposit/withdrawal.
    pub fn simple(
        id: Uuid,
        deal_id: Uuid,
        transaction_type: TransactionType,
        party_id: Uuid,
        amount: Decimal,
        description: Option<String>,
    ) -> Self {
        let (from, to) = match transaction_type {
            TransactionType::Deposit => (None, Some(party_id)),
            TransactionType::Withdrawal => (Some(party_id), None),
            _ => (Some(party_id), Some(party_id)),
        };
        Self::new(
            id,
            deal_id,
            transaction_type,
            from,
            to,
            amount,
            description,
            TransactionStatus::Verified,
            None,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_type_round_trips() {
        for ty in [
            TransactionType::Deposit,
            TransactionType::Withdrawal,
            TransactionType::EscrowHold,
            TransactionType::EscrowRelease,
            TransactionType::Fee,
            TransactionType::Adjustment,
        ] {
            let s = ty.as_str();
            assert_eq!(TransactionType::try_from(s).unwrap(), ty);
        }
    }

    #[test]
    fn transaction_status_round_trips() {
        for status in [
            TransactionStatus::Pending,
            TransactionStatus::Verified,
            TransactionStatus::Complete,
            TransactionStatus::Rejected,
        ] {
            let s = status.as_str();
            assert_eq!(TransactionStatus::try_from(s).unwrap(), status);
        }
    }

    #[test]
    fn simple_deposit_points_to_party() {
        let party_id = Uuid::now_v7();
        let deal_id = Uuid::now_v7();
        let txn = Transaction::simple(
            Uuid::now_v7(),
            deal_id,
            TransactionType::Deposit,
            party_id,
            Decimal::from(100),
            None,
        );

        assert_eq!(txn.deal_id, deal_id);
        assert_eq!(txn.from_party_id, None);
        assert_eq!(txn.to_party_id, Some(party_id));
        assert_eq!(txn.status, TransactionStatus::Verified);
    }

    #[test]
    fn simple_withdrawal_points_from_party() {
        let party_id = Uuid::now_v7();
        let txn = Transaction::simple(
            Uuid::now_v7(),
            Uuid::now_v7(),
            TransactionType::Withdrawal,
            party_id,
            Decimal::from(50),
            None,
        );

        assert_eq!(txn.from_party_id, Some(party_id));
        assert_eq!(txn.to_party_id, None);
    }
}
