use async_trait::async_trait;
use domain::entities::{
    Currency, DealWallet, PlatformWallet, Transaction, TransactionStatus, TransactionType,
};
use domain::errors::DomainError;
use domain::repositories::{TransactionFilters, WalletRepository};
use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, PgPool};
use uuid::Uuid;

pub struct PostgresWalletRepository {
    pool: PgPool,
}

impl PostgresWalletRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WalletRepository for PostgresWalletRepository {
    async fn create(&self, wallet: &PlatformWallet) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO platform_wallets (
                id, party_id, balance, escrow_balance, pending_balance,
                total_deposited, total_withdrawn, currency, is_active,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            wallet.id,
            wallet.party_id,
            wallet.balance,
            wallet.escrow_balance,
            wallet.pending_balance,
            wallet.total_deposited,
            wallet.total_withdrawn,
            wallet.currency.as_str(),
            wallet.is_active,
            wallet.created_at,
            wallet.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_party_id(
        &self,
        party_id: Uuid,
    ) -> Result<Option<PlatformWallet>, DomainError> {
        let row = sqlx::query_as!(
            WalletRow,
            r#"
            SELECT
                id as "id!",
                party_id as "party_id!",
                balance as "balance!",
                escrow_balance as "escrow_balance!",
                pending_balance as "pending_balance!",
                total_deposited as "total_deposited!",
                total_withdrawn as "total_withdrawn!",
                currency as "currency!",
                is_active as "is_active!",
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM platform_wallets
            WHERE party_id = $1
            "#,
            party_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_wallet_from_row))
    }

    async fn update(&self, wallet: &PlatformWallet) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE platform_wallets
            SET balance = $1,
                escrow_balance = $2,
                pending_balance = $3,
                total_deposited = $4,
                total_withdrawn = $5,
                is_active = $6,
                updated_at = $7
            WHERE id = $8
            "#,
            wallet.balance,
            wallet.escrow_balance,
            wallet.pending_balance,
            wallet.total_deposited,
            wallet.total_withdrawn,
            wallet.is_active,
            wallet.updated_at,
            wallet.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn record_transaction(
        &self,
        wallet: &PlatformWallet,
        transaction: &Transaction,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        sqlx::query!(
            r#"
            UPDATE platform_wallets
            SET balance = $1,
                escrow_balance = $2,
                pending_balance = $3,
                total_deposited = $4,
                total_withdrawn = $5,
                updated_at = $6
            WHERE id = $7
            "#,
            wallet.balance,
            wallet.escrow_balance,
            wallet.pending_balance,
            wallet.total_deposited,
            wallet.total_withdrawn,
            wallet.updated_at,
            wallet.id
        )
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;

        sqlx::query!(
            r#"
            INSERT INTO transactions (
                id, deal_id, agreement_id, milestone_id, transaction_type,
                from_party_id, to_party_id, amount, currency, description,
                status, payment_method, external_reference, requires_approval,
                approvals_required, approvals_received, executed_at, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            "#,
            transaction.id,
            transaction.deal_id,
            transaction.agreement_id,
            transaction.milestone_id,
            transaction.transaction_type.as_str(),
            transaction.from_party_id,
            transaction.to_party_id,
            transaction.amount,
            transaction.currency.as_str(),
            transaction.description,
            transaction.status.as_str(),
            transaction.payment_method,
            transaction.external_reference,
            transaction.requires_approval,
            transaction.approvals_required,
            transaction.approvals_received,
            transaction.executed_at,
            transaction.created_at
        )
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;

        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    async fn find_transactions(
        &self,
        party_id: Uuid,
        filters: &TransactionFilters,
    ) -> Result<Vec<Transaction>, DomainError> {
        let rows = sqlx::query_as!(
            TransactionRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                agreement_id,
                milestone_id,
                transaction_type as "transaction_type!",
                from_party_id,
                to_party_id,
                amount as "amount!",
                currency as "currency!",
                description,
                status as "status!",
                payment_method,
                external_reference,
                requires_approval as "requires_approval!",
                approvals_required as "approvals_required!",
                approvals_received as "approvals_received!",
                executed_at,
                created_at as "created_at!"
            FROM transactions
            WHERE (from_party_id = $1 OR to_party_id = $1)
              AND ($2::uuid IS NULL OR deal_id = $2)
              AND ($3::text IS NULL OR status = $3)
              AND ($4::text IS NULL OR transaction_type = $4)
            ORDER BY created_at ASC
            LIMIT $5
            OFFSET $6
            "#,
            party_id,
            filters.deal_id,
            filters.status,
            filters.transaction_type,
            filters.limit,
            filters.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_transaction_from_row).collect())
    }

    async fn count_transactions(
        &self,
        party_id: Uuid,
        filters: &TransactionFilters,
    ) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM transactions
            WHERE (from_party_id = $1 OR to_party_id = $1)
              AND ($2::uuid IS NULL OR deal_id = $2)
              AND ($3::text IS NULL OR status = $3)
              AND ($4::text IS NULL OR transaction_type = $4)
            "#,
            party_id,
            filters.deal_id,
            filters.status,
            filters.transaction_type
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn compute_deal_wallet(
        &self,
        party_id: Uuid,
        deal_id: Uuid,
    ) -> Result<Option<DealWallet>, DomainError> {
        let rows = sqlx::query_as!(
            TransactionRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                agreement_id,
                milestone_id,
                transaction_type as "transaction_type!",
                from_party_id,
                to_party_id,
                amount as "amount!",
                currency as "currency!",
                description,
                status as "status!",
                payment_method,
                external_reference,
                requires_approval as "requires_approval!",
                approvals_required as "approvals_required!",
                approvals_received as "approvals_received!",
                executed_at,
                created_at as "created_at!"
            FROM transactions
            WHERE deal_id = $1
              AND (from_party_id = $2 OR to_party_id = $2)
            ORDER BY created_at ASC
            "#,
            deal_id,
            party_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        if rows.is_empty() {
            return Ok(None);
        }

        let mut dw = DealWallet::new(party_id, deal_id, Currency::Points);
        for row in rows {
            let t = build_transaction_from_row(row);
            match t.transaction_type {
                TransactionType::Deposit if t.to_party_id == Some(party_id) => {
                    dw.deposited += t.amount;
                    dw.contributed += t.amount;
                }
                TransactionType::Withdrawal if t.from_party_id == Some(party_id) => {
                    dw.withdrawn += t.amount;
                    dw.contributed -= t.amount;
                }
                TransactionType::EscrowHold if t.from_party_id == Some(party_id) => {
                    dw.held_in_escrow += t.amount;
                    dw.contributed += t.amount;
                }
                TransactionType::EscrowRelease if t.to_party_id == Some(party_id) => {
                    dw.released += t.amount;
                    dw.held_in_escrow -= t.amount;
                }
                TransactionType::Fee if t.from_party_id == Some(party_id) => {
                    dw.fees_paid += t.amount;
                }
                TransactionType::Adjustment if t.to_party_id == Some(party_id) => {
                    dw.released += t.amount;
                }
                TransactionType::Adjustment if t.from_party_id == Some(party_id) => {
                    dw.contributed += t.amount;
                }
                _ => {}
            }
        }
        dw.net_position = dw.released + dw.withdrawn - dw.fees_paid - dw.contributed;
        Ok(Some(dw))
    }
}

#[derive(sqlx::FromRow)]
struct WalletRow {
    id: Uuid,
    party_id: Uuid,
    balance: Decimal,
    escrow_balance: Decimal,
    pending_balance: Decimal,
    total_deposited: Decimal,
    total_withdrawn: Decimal,
    currency: String,
    is_active: bool,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn build_wallet_from_row(row: WalletRow) -> PlatformWallet {
    PlatformWallet {
        id: row.id,
        party_id: row.party_id,
        balance: row.balance,
        escrow_balance: row.escrow_balance,
        pending_balance: row.pending_balance,
        total_deposited: row.total_deposited,
        total_withdrawn: row.total_withdrawn,
        currency: Currency::try_from(row.currency.as_str()).unwrap_or(Currency::Points),
        is_active: row.is_active,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

#[derive(sqlx::FromRow)]
struct TransactionRow {
    id: Uuid,
    deal_id: Uuid,
    agreement_id: Option<Uuid>,
    milestone_id: Option<Uuid>,
    transaction_type: String,
    from_party_id: Option<Uuid>,
    to_party_id: Option<Uuid>,
    amount: Decimal,
    currency: String,
    description: Option<String>,
    status: String,
    payment_method: Option<String>,
    external_reference: Option<String>,
    requires_approval: bool,
    approvals_required: i32,
    approvals_received: i32,
    executed_at: Option<time::OffsetDateTime>,
    created_at: time::OffsetDateTime,
}

fn build_transaction_from_row(row: TransactionRow) -> Transaction {
    Transaction {
        id: row.id,
        deal_id: row.deal_id,
        agreement_id: row.agreement_id,
        milestone_id: row.milestone_id,
        transaction_type: TransactionType::try_from(row.transaction_type.as_str())
            .unwrap_or(TransactionType::Deposit),
        from_party_id: row.from_party_id,
        to_party_id: row.to_party_id,
        amount: row.amount,
        currency: Currency::try_from(row.currency.as_str()).unwrap_or(Currency::Points),
        description: row.description,
        status: TransactionStatus::try_from(row.status.as_str())
            .unwrap_or(TransactionStatus::Verified),
        payment_method: row.payment_method,
        external_reference: row.external_reference,
        requires_approval: row.requires_approval,
        approvals_required: row.approvals_required,
        approvals_received: row.approvals_received,
        executed_at: row.executed_at,
        created_at: row.created_at,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
