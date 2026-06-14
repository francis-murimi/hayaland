use crate::errors::DomainError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Currency used within the platform wallet subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Currency {
    #[default]
    Points,
}

impl Currency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Currency::Points => "POINTS",
        }
    }
}

impl TryFrom<&str> for Currency {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "POINTS" => Ok(Currency::Points),
            _ => Err(DomainError::Validation(vec![format!(
                "unknown currency: {value}"
            )])),
        }
    }
}

/// A party's wallet container.
///
/// There is exactly one `PlatformWallet` row per party. It stores aggregate
/// balances; per-deal balances are derived from `Transaction` rows filtered by
/// `deal_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformWallet {
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

impl PlatformWallet {
    /// Create a new, empty wallet container for a party.
    pub fn new(id: Uuid, party_id: Uuid) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            party_id,
            balance: Decimal::ZERO,
            escrow_balance: Decimal::ZERO,
            pending_balance: Decimal::ZERO,
            total_deposited: Decimal::ZERO,
            total_withdrawn: Decimal::ZERO,
            currency: Currency::Points,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    fn ensure_active(&self) -> Result<(), DomainError> {
        if !self.is_active {
            Err(DomainError::Validation(vec![
                "wallet is inactive".to_string()
            ]))
        } else {
            Ok(())
        }
    }

    fn validate_amount(amount: Decimal) -> Result<(), DomainError> {
        if amount <= Decimal::ZERO {
            Err(DomainError::Validation(vec![
                "amount must be positive".to_string()
            ]))
        } else {
            Ok(())
        }
    }

    /// Record an external deposit into the wallet container.
    pub fn deposit(&mut self, amount: Decimal) -> Result<(), DomainError> {
        Self::validate_amount(amount)?;
        self.ensure_active()?;
        self.balance += amount;
        self.total_deposited += amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    /// Record a withdrawal from the wallet container.
    pub fn withdraw(&mut self, amount: Decimal) -> Result<(), DomainError> {
        Self::validate_amount(amount)?;
        self.ensure_active()?;
        if amount > self.balance {
            return Err(DomainError::Validation(vec![
                "insufficient balance".to_string()
            ]));
        }
        self.balance -= amount;
        self.total_withdrawn += amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    /// Move available balance into escrow (e.g. when a consumer commits to a deal).
    pub fn hold_escrow(&mut self, amount: Decimal) -> Result<(), DomainError> {
        Self::validate_amount(amount)?;
        self.ensure_active()?;
        if amount > self.balance {
            return Err(DomainError::Validation(vec![
                "insufficient balance".to_string()
            ]));
        }
        self.balance -= amount;
        self.escrow_balance += amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    /// Release escrow back to available balance for the same party.
    pub fn release_escrow_to_self(&mut self, amount: Decimal) -> Result<(), DomainError> {
        Self::validate_amount(amount)?;
        self.ensure_active()?;
        if amount > self.escrow_balance {
            return Err(DomainError::Validation(vec![
                "insufficient escrow balance".to_string()
            ]));
        }
        self.escrow_balance -= amount;
        self.balance += amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    /// Deduct a fee from available balance.
    pub fn deduct_fee_from_balance(&mut self, amount: Decimal) -> Result<(), DomainError> {
        Self::validate_amount(amount)?;
        self.ensure_active()?;
        if amount > self.balance {
            return Err(DomainError::Validation(vec![
                "insufficient balance".to_string()
            ]));
        }
        self.balance -= amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    /// Deduct a fee from escrow balance.
    pub fn deduct_fee_from_escrow(&mut self, amount: Decimal) -> Result<(), DomainError> {
        Self::validate_amount(amount)?;
        self.ensure_active()?;
        if amount > self.escrow_balance {
            return Err(DomainError::Validation(vec![
                "insufficient escrow balance".to_string()
            ]));
        }
        self.escrow_balance -= amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn mark_inactive(&mut self) {
        self.is_active = false;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn mark_active(&mut self) {
        self.is_active = true;
        self.updated_at = OffsetDateTime::now_utc();
    }

    /// Move available balance into pending while awaiting approval.
    pub fn hold_pending(&mut self, amount: Decimal) -> Result<(), DomainError> {
        Self::validate_amount(amount)?;
        self.ensure_active()?;
        if amount > self.balance {
            return Err(DomainError::Validation(vec![
                "insufficient balance".to_string()
            ]));
        }
        self.balance -= amount;
        self.pending_balance += amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    /// Release pending balance back to available balance (on rejection).
    pub fn release_pending(&mut self, amount: Decimal) -> Result<(), DomainError> {
        Self::validate_amount(amount)?;
        self.ensure_active()?;
        if amount > self.pending_balance {
            return Err(DomainError::Validation(vec![
                "insufficient pending balance".to_string(),
            ]));
        }
        self.pending_balance -= amount;
        self.balance += amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    /// Commit pending balance to escrow (e.g. approved escrow hold).
    pub fn commit_pending_to_escrow(&mut self, amount: Decimal) -> Result<(), DomainError> {
        Self::validate_amount(amount)?;
        self.ensure_active()?;
        if amount > self.pending_balance {
            return Err(DomainError::Validation(vec![
                "insufficient pending balance".to_string(),
            ]));
        }
        self.pending_balance -= amount;
        self.escrow_balance += amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    /// Commit pending balance back to available balance (e.g. approved deposit/release).
    pub fn commit_pending_to_balance(&mut self, amount: Decimal) -> Result<(), DomainError> {
        Self::validate_amount(amount)?;
        self.ensure_active()?;
        if amount > self.pending_balance {
            return Err(DomainError::Validation(vec![
                "insufficient pending balance".to_string(),
            ]));
        }
        self.pending_balance -= amount;
        self.balance += amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    /// Debit escrow for a cross-party release.
    pub fn debit_escrow(&mut self, amount: Decimal) -> Result<(), DomainError> {
        Self::validate_amount(amount)?;
        self.ensure_active()?;
        if amount > self.escrow_balance {
            return Err(DomainError::Validation(vec![
                "insufficient escrow balance".to_string()
            ]));
        }
        self.escrow_balance -= amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    /// Credit available balance for a cross-party release.
    pub fn credit_balance(&mut self, amount: Decimal) -> Result<(), DomainError> {
        Self::validate_amount(amount)?;
        self.ensure_active()?;
        self.balance += amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }
}

/// A read-only, per-deal view of a party's wallet.
///
/// This value object is computed from `Transaction` rows for a single
/// `(party_id, deal_id)` pair. It is not persisted directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DealWallet {
    pub party_id: Uuid,
    pub deal_id: Uuid,
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

impl DealWallet {
    pub fn new(party_id: Uuid, deal_id: Uuid, currency: Currency) -> Self {
        Self {
            party_id,
            deal_id,
            deposited: Decimal::ZERO,
            withdrawn: Decimal::ZERO,
            contributed: Decimal::ZERO,
            held_in_escrow: Decimal::ZERO,
            released: Decimal::ZERO,
            fees_paid: Decimal::ZERO,
            pending: Decimal::ZERO,
            net_position: Decimal::ZERO,
            currency,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_wallet_starts_empty_and_active() {
        let party_id = Uuid::now_v7();
        let wallet = PlatformWallet::new(Uuid::now_v7(), party_id);

        assert_eq!(wallet.party_id, party_id);
        assert_eq!(wallet.balance, Decimal::ZERO);
        assert_eq!(wallet.escrow_balance, Decimal::ZERO);
        assert_eq!(wallet.pending_balance, Decimal::ZERO);
        assert!(wallet.is_active);
        assert_eq!(wallet.currency, Currency::Points);
    }

    #[test]
    fn deposit_increases_balance_and_total_deposited() {
        let mut wallet = PlatformWallet::new(Uuid::now_v7(), Uuid::now_v7());
        wallet.deposit(Decimal::from(100)).unwrap();

        assert_eq!(wallet.balance, Decimal::from(100));
        assert_eq!(wallet.total_deposited, Decimal::from(100));
    }

    #[test]
    fn deposit_rejects_non_positive_amount() {
        let mut wallet = PlatformWallet::new(Uuid::now_v7(), Uuid::now_v7());
        assert!(wallet.deposit(Decimal::ZERO).is_err());
        assert!(wallet.deposit(Decimal::from(-10)).is_err());
    }

    #[test]
    fn withdrawal_requires_sufficient_balance() {
        let mut wallet = PlatformWallet::new(Uuid::now_v7(), Uuid::now_v7());
        wallet.deposit(Decimal::from(50)).unwrap();

        assert!(wallet.withdraw(Decimal::from(100)).is_err());
        wallet.withdraw(Decimal::from(30)).unwrap();

        assert_eq!(wallet.balance, Decimal::from(20));
        assert_eq!(wallet.total_withdrawn, Decimal::from(30));
    }

    #[test]
    fn inactive_wallet_rejects_mutations() {
        let mut wallet = PlatformWallet::new(Uuid::now_v7(), Uuid::now_v7());
        wallet.mark_inactive();

        assert!(wallet.deposit(Decimal::from(10)).is_err());
        assert!(wallet.withdraw(Decimal::from(10)).is_err());
    }

    #[test]
    fn hold_escrow_moves_balance_to_escrow() {
        let mut wallet = PlatformWallet::new(Uuid::now_v7(), Uuid::now_v7());
        wallet.deposit(Decimal::from(100)).unwrap();
        wallet.hold_escrow(Decimal::from(60)).unwrap();

        assert_eq!(wallet.balance, Decimal::from(40));
        assert_eq!(wallet.escrow_balance, Decimal::from(60));
    }

    #[test]
    fn hold_escrow_rejects_overdraft() {
        let mut wallet = PlatformWallet::new(Uuid::now_v7(), Uuid::now_v7());
        wallet.deposit(Decimal::from(10)).unwrap();
        assert!(wallet.hold_escrow(Decimal::from(20)).is_err());
    }

    #[test]
    fn release_escrow_to_self_moves_funds_back() {
        let mut wallet = PlatformWallet::new(Uuid::now_v7(), Uuid::now_v7());
        wallet.deposit(Decimal::from(100)).unwrap();
        wallet.hold_escrow(Decimal::from(60)).unwrap();
        wallet.release_escrow_to_self(Decimal::from(25)).unwrap();

        assert_eq!(wallet.balance, Decimal::from(65));
        assert_eq!(wallet.escrow_balance, Decimal::from(35));
    }

    #[test]
    fn fee_deduction_respects_balance_and_escrow() {
        let mut wallet = PlatformWallet::new(Uuid::now_v7(), Uuid::now_v7());
        wallet.deposit(Decimal::from(100)).unwrap();
        wallet.hold_escrow(Decimal::from(60)).unwrap();

        wallet.deduct_fee_from_balance(Decimal::from(5)).unwrap();
        assert_eq!(wallet.balance, Decimal::from(35));

        wallet.deduct_fee_from_escrow(Decimal::from(10)).unwrap();
        assert_eq!(wallet.escrow_balance, Decimal::from(50));

        assert!(wallet.deduct_fee_from_balance(Decimal::from(100)).is_err());
        assert!(wallet.deduct_fee_from_escrow(Decimal::from(100)).is_err());
    }

    #[test]
    fn pending_balance_hold_release_and_commit() {
        let mut wallet = PlatformWallet::new(Uuid::now_v7(), Uuid::now_v7());
        wallet.deposit(Decimal::from(100)).unwrap();

        wallet.hold_pending(Decimal::from(40)).unwrap();
        assert_eq!(wallet.balance, Decimal::from(60));
        assert_eq!(wallet.pending_balance, Decimal::from(40));

        wallet.commit_pending_to_escrow(Decimal::from(25)).unwrap();
        assert_eq!(wallet.pending_balance, Decimal::from(15));
        assert_eq!(wallet.escrow_balance, Decimal::from(25));

        wallet.release_pending(Decimal::from(15)).unwrap();
        assert_eq!(wallet.balance, Decimal::from(75));
        assert_eq!(wallet.pending_balance, Decimal::ZERO);
    }

    #[test]
    fn pending_hold_rejects_overdraft_and_inactive_wallet() {
        let mut wallet = PlatformWallet::new(Uuid::now_v7(), Uuid::now_v7());
        wallet.deposit(Decimal::from(10)).unwrap();
        assert!(wallet.hold_pending(Decimal::from(20)).is_err());

        wallet.mark_inactive();
        assert!(wallet.hold_pending(Decimal::from(5)).is_err());
    }

    #[test]
    fn debit_and_credit_for_cross_party_release() {
        let mut source = PlatformWallet::new(Uuid::now_v7(), Uuid::now_v7());
        let mut recipient = PlatformWallet::new(Uuid::now_v7(), Uuid::now_v7());
        source.deposit(Decimal::from(100)).unwrap();
        source.hold_escrow(Decimal::from(60)).unwrap();

        source.debit_escrow(Decimal::from(40)).unwrap();
        recipient.credit_balance(Decimal::from(40)).unwrap();

        assert_eq!(source.escrow_balance, Decimal::from(20));
        assert_eq!(recipient.balance, Decimal::from(40));
        assert!(source.debit_escrow(Decimal::from(100)).is_err());
    }
}
