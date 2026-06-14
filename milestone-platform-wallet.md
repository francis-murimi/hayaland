# Milestone: Platform Wallet

## Overview
Deliver the Platform Wallet feature for the Hayaland workspace: one persistent wallet container per party, with per-deal sub-wallets derived from `transactions.deal_id`. All transactions (deposits, withdrawals, escrow, fees, adjustments) are tied to a `deal_id`.

## Goals
- [x] One `platform_wallets` row per party (container).
- [x] Per-deal wallet views computed from `transactions` filtered by `deal_id`.
- [x] All ledger movements require a `deal_id`.
- [x] Wallet auto-created when a party is created.
- [x] >80% test coverage with green formatter/linter/sqlx checks.

## Deliverables

### Domain
- `PlatformWallet` aggregate with balance, escrow, pending, totals, and currency.
- `DealWallet` read-only per-deal value object.
- `Transaction` / `TransactionType` / `TransactionStatus` entities.
- `WalletRepository` outbound port and `TransactionFilters`.

### Application Use Cases
| Use Case | Purpose |
|----------|---------|
| `CreateWallet` | Auto-create party wallet container |
| `DepositPoints` | Record an external deposit for a deal |
| `WithdrawPoints` | Record a withdrawal for a deal |
| `HoldEscrow` | Move available balance into escrow |
| `ReleaseEscrow` | Release escrow back to available balance |
| `DeductFee` | Deduct a fee from balance or escrow |
| `RecordAdjustment` | Administrative credit/debit adjustment |
| `GetWallet` | Read party wallet container |
| `GetDealWallet` | Compute per-deal sub-wallet |
| `ListWalletTransactions` | List party transactions with filters |
| `ListDealTransactions` | List transactions for a single deal |

### Infrastructure
- `PostgresWalletRepository` implementing the repository port.
- Atomic `record_transaction(wallet, transaction)` updating the container and ledger.
- SQL queries for filtered transaction listing, counting, and per-deal aggregation.

### API Routes
| Method | Path | Handler |
|--------|------|---------|
| `GET` | `/api/v1/parties/{id}/wallet` | Get wallet container |
| `POST` | `/api/v1/parties/{id}/wallet/deposits` | Deposit points |
| `POST` | `/api/v1/parties/{id}/wallet/withdrawals` | Withdraw points |
| `GET` | `/api/v1/parties/{id}/wallet/transactions` | List wallet transactions |
| `GET` | `/api/v1/parties/{party_id}/deals/{deal_id}/wallet` | Get deal sub-wallet |
| `GET` | `/api/v1/parties/{party_id}/deals/{deal_id}/transactions` | List deal transactions |

## Quality Gate
- `cargo test` — all tests pass.
- `cargo fmt --check` — clean.
- `cargo clippy -- -D warnings` — clean.
- `cargo sqlx prepare --workspace --check` — clean.
- `cargo llvm-cov --workspace` — **84.38%** line coverage.

## Notes
- `CreateParty` now requires a `WalletRepository`; production wiring uses `CreateParty::new_with_wallet(...)` so every new party gets a wallet automatically.
- `HoldEscrow`, `ReleaseEscrow`, `DeductFee`, and `RecordAdjustment` are implemented, unit-tested, and available in `AppState` for future admin/escrow workflow routes.
- Transaction approval workflow scaffolding (`transaction_approvals` table) is present; multi-sig approval logic is out of scope for this milestone.
