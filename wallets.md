# Wallets — Hayaland 3-Party Deal Platform

> **Scope:** This document specifies the **Platform Wallet** subsystem: how every Party gets a wallet, how wallet balances are maintained, and how wallet movements are driven by the point-based transaction ledger.
>
> **Audience:** Backend engineers, product owners, and API consumers.
>
> **Based on:** `3partydeal.pdf` Software Design Document, `hayaland-deal-plan.md`, `deal-plan.md`, `party-guide.md`, `trust-score.md`, `AGENTS.md`, and the existing `hayaland` Rust codebase.

---

## 1. Goals

1. **One wallet container per Party.** Every `Party` has exactly one `platform_wallets` row. The wallet is created automatically when the party is created. This row is a **container**; it does not hold deal-specific balances directly.
2. **Per-deal sub-wallets.** Within a party's wallet container, every deal the party participates in has a logical **per-deal wallet** (sub-wallet). The sub-wallet balances are derived from `transactions` filtered by `deal_id` and party.
3. **Point-based ledger.** All values are stored in `POINTS`, an internal platform unit. Wallets do not hold fiat; they mirror real-world settlements recorded via `transactions`.
3. **Per-deal traceability.** Every value-moving transaction is tied to a `deal_id`. A Party can view, per deal, how many points they contributed, hold in escrow, have released, paid in fees, or are owed.
5. **Escrow support.** The wallet container tracks aggregate `balance`, `escrow_balance`, and `pending_balance`. Each per-deal sub-wallet tracks its own contributed, escrowed, released, fee, and net amounts.
6. **Auditability.** Every wallet mutation is paired with a `Transaction` row (deposits, withdrawals, escrow holds/releases, fees, adjustments).
7. **Multi-party foundation.** Wallet operations that move value between parties create `PENDING` transactions that require approval by every involved party before the ledger is mutated.
8. **No code changes to existing modules.** The `platform_wallets` table already exists in the migration set; this document describes the application code that should be added around it.

---

## 2. Wallets at a Glance

| Property | Value |
|---|---|
| Currency | `POINTS` (platform-internal unit) |
| Wallet model | One wallet **container** per party; logical per-deal **sub-wallets** derived from `transactions` |
| Balance components | Container: `balance`, `escrow_balance`, `pending_balance`. Sub-wallet: contributed, held, released, fees, net. |
| Creation | Container auto-created when a `Party` is created. No new container is created per deal. |
| Deletion | Container cascade-deleted when the parent `Party` is removed. Per-deal sub-wallets disappear with their transactions. |
| Status | Container `is_active` mirrors the parent party status |
| Storage | `platform_wallets` (one row per party) + `transactions` (per-deal ledger) |

A Party's wallet container is private. Only members of the party (and platform admins) can view or operate on it. The container holds the aggregate balances; the **per-deal sub-wallet** is the view a party actually uses when tracking the financial progress of an active deal. Because sub-wallets are derived from `transactions`, no new wallet row needs to be created when a party enters a new deal.

---

## 3. Balance Types

### 3.1 Container balances (`platform_wallets`)

| Balance | Meaning | Can be withdrawn? |
|---|---|---|
| `balance` | Available platform points across all deals, ready for new deals or withdrawal. | Yes |
| `escrow_balance` | Aggregate points locked across all active deals. Released when milestones are verified or refunded on cancellation. | No |
| `pending_balance` | Aggregate points tied to transactions awaiting multi-party approval across all deals. | No |

### 3.2 Per-deal sub-wallet balances (derived from `transactions`)

| Balance | Meaning |
|---|---|
| `contributed` | Points the party put into the deal (deposits + escrow holds). |
| `heldInEscrow` | Points still held in escrow for this deal. |
| `released` | Points released to the party from escrow (milestone payments, refunds). |
| `feesPaid` | Platform fees paid by the party for this deal. |
| `pending` | Points awaiting approval for this deal. |
| `netPosition` | `released + withdrawals - fees - contributed` for the deal. |

**Per-deal view:**

For any `(party_id, deal_id)` pair the system can compute:

```text
deal_deposited     = SUM(amount) WHERE party is beneficiary and type = DEPOSIT
deal_withdrawn     = SUM(amount) WHERE party is source      and type = WITHDRAWAL
deal_contributed   = deal_deposited + SUM(amount) WHERE party is source and type = ESCROW_HOLD
deal_held_in_escrow= SUM(amount) WHERE party is source      and type = ESCROW_HOLD
                   - SUM(amount) WHERE party is beneficiary and type = ESCROW_RELEASE
deal_released      = SUM(amount) WHERE party is beneficiary and type = ESCROW_RELEASE
deal_fees_paid     = SUM(amount) WHERE party is source      and type = FEE
deal_net_position  = deal_released + deal_withdrawn - deal_fees_paid - deal_contributed
```

`refunds` are recorded as `ESCROW_RELEASE` or `ADJUSTMENT` rows and therefore already feed `deal_released` / `deal_contributed`.

These per-deal figures are derived from `transactions` filtered by `deal_id` and the party's involvement. They do not require a separate table, but a materialized `deal_wallet_balances` cache can be added later if query volume requires it.

**Invariants:**

- `balance >= 0`, `escrow_balance >= 0`, `pending_balance >= 0` at all times.
- A withdrawal or escrow hold can only debit `balance`.
- A transaction approval atomically moves points from `pending_balance` to the final destination (`balance` or `escrow_balance`).
- If a transaction is rejected, the `pending_balance` is returned to `balance`.
- **Every transaction, including deposits and withdrawals, must include a `deal_id`.** There are no system-level wallet movements.

---

## 4. Wallet Lifecycle

### 4.1 Creation

When `CreateParty` succeeds, the application layer must also call `CreateWallet` (or the Postgres repository must insert the row in the same unit of work). This creates the **wallet container** for the party. The container starts with all aggregate balances at zero. No per-deal wallet row is created at this stage — sub-wallets materialise automatically as transactions are recorded against each deal.

```text
balance = 0
escrow_balance = 0
pending_balance = 0
total_deposited = 0
total_withdrawn = 0
currency = 'POINTS'
is_active = true
```

### 4.2 Activation / Deactivation

- When a party is soft-deleted (`is_active = false`), its wallet becomes inactive.
- An inactive wallet cannot be debited, credited, or used in new deals.
- Admins can reactivate a party and its wallet together.

### 4.3 Deletion

- Hard-deletion of a party cascades to the wallet row via `ON DELETE CASCADE`.
- Historical `transactions` remain because they reference `deal_id`, not the wallet directly.

---

## 5. Operations

The wallet subsystem exposes the following operations. Every operation that changes a balance creates a `Transaction` row. **All operations, including deposits and withdrawals, require a `deal_id`** and validate that the acting party participates in that deal.

| Operation | `deal_id` | Effect on wallet | Creates transaction |
|---|---|---|---|
| `DepositPoints` | Required | `balance += amount`, `total_deposited += amount` | `DEPOSIT` |
| `WithdrawPoints` | Required | `balance -= amount`, `total_withdrawn += amount` | `WITHDRAWAL` |
| `HoldEscrow` | Required | `balance -= amount`, `escrow_balance += amount` | `ESCROW_HOLD` |
| `ReleaseEscrow` | Required | `escrow_balance -= amount`, `balance += amount` (or `to_party.balance += amount`) | `ESCROW_RELEASE` |
| `DeductFee` | Required | `balance -= amount` or `escrow_balance -= amount` | `FEE` |
| `RecordAdjustment` | Required | Context-specific debit/credit | `ADJUSTMENT` |

All mutating operations are performed inside a database transaction together with the matching `Transaction` insert (and any `transaction_approvals` if approval is required).

---

## 6. Database Schema (Existing)

The wallet container is implemented by the existing `platform_wallets` table (migration `20260613015000_create_matches_wallets_transactions.sql`). There is **no separate per-deal wallet table**; per-deal sub-wallets are derived from `transactions`.

```sql
CREATE TABLE IF NOT EXISTS platform_wallets (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL UNIQUE REFERENCES parties(id) ON DELETE CASCADE,
    balance DECIMAL NOT NULL DEFAULT 0, -- aggregate available platform points
    escrow_balance DECIMAL NOT NULL DEFAULT 0, -- aggregate points held in escrow
    pending_balance DECIMAL NOT NULL DEFAULT 0, -- aggregate points awaiting approval
    total_deposited DECIMAL NOT NULL DEFAULT 0,
    total_withdrawn DECIMAL NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'POINTS',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

The related `transactions` and `transaction_approvals` tables are also already in place. `transactions.deal_id` is what partitions a party's activity into per-deal sub-wallets:

```sql
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY,
    deal_id UUID REFERENCES deals(id),
    agreement_id UUID REFERENCES agreements(id),
    milestone_id UUID REFERENCES milestones(id),
    transaction_type TEXT NOT NULL,
    from_party_id UUID REFERENCES parties(id),
    to_party_id UUID REFERENCES parties(id),
    amount DECIMAL NOT NULL,
    currency TEXT NOT NULL DEFAULT 'POINTS',
    description TEXT,
    status TEXT NOT NULL DEFAULT 'PENDING',
    payment_method TEXT,
    external_reference TEXT,
    requires_approval BOOLEAN NOT NULL DEFAULT true,
    approvals_required INTEGER NOT NULL DEFAULT 2,
    approvals_received INTEGER NOT NULL DEFAULT 0,
    executed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS transaction_approvals (
    id UUID PRIMARY KEY,
    transaction_id UUID NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
    party_id UUID NOT NULL REFERENCES parties(id),
    approved_by_user_id UUID NOT NULL REFERENCES users(id),
    decision TEXT NOT NULL CHECK (decision IN ('APPROVED','REJECTED')),
    comment TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (transaction_id, party_id)
);
```

> **Note:** The full transaction approval workflow is specified in `trust-score.md` and will be detailed further in a dedicated `transactions.md`. This document focuses on the wallet side.

---

## 7. Domain & Application Additions

To implement this document in the existing hexagonal codebase, the following modules are recommended (no existing source code is changed).

### 7.1 `crates/domain`

```text
domain/src/entities/
  └── wallet.rs                 # PlatformWallet (container), WalletStatus, Currency
                                # DealWallet (computed value object, no persistence)

domain/src/repositories/
  └── wallet_repository.rs      # WalletRepository outbound port
```

`PlatformWallet` represents the container. `DealWallet` is a read-only value object returned by `get_deal_wallet.rs` and contains the per-deal figures described in §3.2.

`PlatformWallet` should enforce invariants at the domain layer:

- No negative balances.
- Currency is always `POINTS`.
- Wallet container is active to allow debits/credits.
- Per-deal sub-wallets are not persisted; they are derived from `transactions`.

### 7.2 `crates/application`

```text
application/src/payments/
  ├── create_wallet.rs          # Internal use case invoked by CreateParty
  ├── get_wallet.rs             # Read a party's global wallet
  ├── get_deal_wallet.rs        # Per-deal wallet view for a party
  ├── deposit_points.rs         # Record an external deposit
  ├── withdraw_points.rs        # Record an external withdrawal
  ├── hold_escrow.rs            # Move balance → escrow for a deal
  ├── release_escrow.rs         # Move escrow → recipient balance
  ├── deduct_fee.rs             # Platform fee deduction
  ├── record_adjustment.rs      # Admin/manual correction
  ├── list_wallet_transactions.rs
  ├── list_deal_transactions.rs
  └── dto.rs
```

### 7.3 `crates/infrastructure`

```text
infrastructure/src/repositories/
  └── postgres_wallet_repository.rs
```

### 7.4 `crates/api`

```text
api/src/routes/
  └── payments.rs

api/src/handlers/payments/
  ├── get_wallet.rs
  ├── deposit_points.rs
  ├── withdraw_points.rs
  ├── list_wallet_transactions.rs
  └── ...
```

---

## 8. API Contracts

All endpoints require a valid JWT `Authorization: Bearer <token>` header. Endpoints that operate on a specific party's wallet also require the `X-Party-ID` header, consistent with the existing deal/party APIs.

### 8.1 Get my wallet container

Returns the party's aggregate wallet container.

```http
GET /api/v1/payments/wallets/me
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Success `200`:

```json
{
  "walletId": "wallet-uuid-1",
  "partyId": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "balance": 18500.00,
  "escrowBalance": 4000.00,
  "pendingBalance": 2000.00,
  "totalDeposited": 75000.00,
  "totalWithdrawn": 56500.00,
  "currency": "POINTS",
  "isActive": true,
  "createdAt": "2026-01-15T10:30:00Z",
  "updatedAt": "2026-06-14T10:05:00Z"
}
```

### 8.2 Get per-deal sub-wallet

Returns the party's sub-wallet for a single deal. The sub-wallet is computed on demand from `transactions` filtered by the party and the deal; no persistent sub-wallet row is read.

```http
GET /api/v1/deals/{dealId}/wallet
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Success `200`:

```json
{
  "dealId": "deal-uuid-1",
  "partyId": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "contributed": 10000.00,
  "heldInEscrow": 6000.00,
  "released": 3000.00,
  "feesPaid": 1000.00,
  "pending": 0.00,
  "netPosition": -8000.00,
  "currency": "POINTS"
}
```

> **Access control:** the caller must be a member of `partyId`, and `partyId` must be a participant in `dealId`. Admins with `admin:deals` or `admin:*` may read any party's per-deal wallet.

### 8.3 Deposit points

Records that the party has performed an external deposit (bank transfer, cash, etc.) and credits the wallet.

```http
POST /api/v1/payments/wallets/me/deposits
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json

{
  "amount": 10000.00,
  "description": "Bank transfer deposit",
  "paymentMethod": "BANK_TRANSFER",
  "externalReference": "wire-ref-12345",
  "dealId": "deal-uuid-1"
}
```

Success `201` with the created `DEPOSIT` transaction.

> The deposit is credited to the party's global `balance` and is recorded against `dealId`. It appears in both the global ledger and the per-deal wallet view.

### 8.4 Withdraw points

Records that the party wants to withdraw available balance to an external account.

```http
POST /api/v1/payments/wallets/me/withdrawals
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json

{
  "amount": 5000.00,
  "description": "Withdrawal to business account",
  "paymentMethod": "BANK_TRANSFER",
  "externalReference": "withdrawal-ref-67890",
  "dealId": "deal-uuid-1"
}
```

Success `201` with the created `WITHDRAWAL` transaction.

> The withdrawal is debited from the party's global `balance` and is recorded against `dealId`. It appears in both the global ledger and the per-deal wallet view.

> **Business rule:** `amount` must be `<= balance`. The withdrawal creates a `PENDING` transaction that typically requires approval by the party itself (and platform ops in the future) before the balance is debited.

### 8.5 List wallet transactions

```http
GET /api/v1/payments/wallets/me/transactions?status=PENDING&dealId=...
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Success `200`:

```json
{
  "transactions": [
    {
      "transactionId": "txn-uuid-1",
      "dealId": "deal-uuid-1",
      "transactionType": "ESCROW_HOLD",
      "amount": 10000.00,
      "currency": "POINTS",
      "status": "VERIFIED",
      "description": "Escrow hold for deal commitment",
      "createdAt": "2026-06-14T10:00:00Z"
    }
  ],
  "total": 1
}
```

### 8.6 List transactions for a specific deal

```http
GET /api/v1/deals/{dealId}/transactions
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Returns only the transactions for the acting party in the given deal, in chronological order. This is the primary view for tracking deal settlement progress.

Success `200`:

```json
{
  "dealId": "deal-uuid-1",
  "transactions": [
    {
      "transactionId": "txn-uuid-1",
      "transactionType": "ESCROW_HOLD",
      "amount": 10000.00,
      "currency": "POINTS",
      "status": "VERIFIED",
      "description": "Consumer escrow hold",
      "createdAt": "2026-06-14T10:00:00Z"
    },
    {
      "transactionId": "txn-uuid-2",
      "transactionType": "ESCROW_RELEASE",
      "amount": 3000.00,
      "currency": "POINTS",
      "status": "VERIFIED",
      "description": "Milestone 1 release to supplier",
      "createdAt": "2026-06-20T10:00:00Z"
    }
  ],
  "total": 2
}
```

### 8.7 Admin adjustment (platform ops)

```http
POST /api/v1/admin/payments/wallets/{partyId}/adjustments
Authorization: Bearer <jwt>
Content-Type: application/json

{
  "amount": 1000.00,
  "description": "Refund for disputed milestone",
  "dealId": "deal-uuid-1"
}
```

Requires scope `admin:payments` or `admin:*`. Success `201` with the created `ADJUSTMENT` transaction.

---

## 9. Validation & Business Rules

| Rule | Layer | Failure |
|---|---|---|
| `amount > 0` | Domain / Application | `Validation` |
| `amount` has at most 2 decimal places | Application | `Validation` |
| Wallet exists for the party | Application | `PartyNotFound` or `WalletNotFound` |
| Wallet is active | Domain / Application | `Validation` |
| Caller is a member of the acting party | Application | `Forbidden` |
| Caller is a participant in the deal | Application | `DealAccessDenied` / `Forbidden` |
| `deal_id` is provided for every transaction | Application | `Validation` |
| Withdrawal / escrow hold `amount <= balance` | Domain / Application | `Validation` |
| Escrow release `amount <= escrow_balance` | Domain / Application | `Validation` |
| Currency is always `POINTS` | Domain | `Validation` |
| Deposit/withdrawal external reference is optional but max 255 chars | Application | `Validation` |

---

## 10. Ledger Mutation Rules

The table below summarises how each operation mutates balances. All mutations happen atomically with the matching `Transaction` row. Deal-scoped rows always set `transactions.deal_id`.

| Operation | `deal_id` | From balance | To balance | `total_deposited` | `total_withdrawn` | Transaction type |
|---|---|---|---|---|---|---|
| Deposit | Required | — | `balance += amount` | `+= amount` | — | `DEPOSIT` |
| Withdrawal | Required | `balance -= amount` | — | — | `+= amount` | `WITHDRAWAL` |
| Escrow hold | Required | `balance -= amount` | `escrow_balance += amount` | — | — | `ESCROW_HOLD` |
| Escrow release to same party | Required | `escrow_balance -= amount` | `balance += amount` | — | — | `ESCROW_RELEASE` |
| Escrow release to another party | Required | `from.escrow_balance -= amount` | `to.balance += amount` | — | — | `ESCROW_RELEASE` |
| Platform fee from balance | Required | `balance -= amount` | — | — | — | `FEE` |
| Platform fee from escrow | Required | `escrow_balance -= amount` | — | — | — | `FEE` |
| Adjustment (credit) | Required | — | `balance += amount` | — | — | `ADJUSTMENT` |
| Adjustment (debit) | Required | `balance -= amount` | — | — | — | `ADJUSTMENT` |

---

## 11. Error Mapping

| Situation | Domain / Application error | HTTP status |
|---|---|---|
| Wallet not found | `WalletNotFound` / `NotFound` | 404 |
| Party not found | `PartyNotFound` | 404 |
| Caller is not a member of the acting party | `Forbidden` | 403 |
| Wallet is inactive | `Validation` | 422 |
| Amount must be positive | `Validation` | 422 |
| Insufficient available balance | `Validation` | 422 |
| Insufficient escrow balance | `Validation` | 422 |
| Currency mismatch | `Validation` | 422 |
| Admin scope missing | `Forbidden` | 403 |

---

## 12. Integration with Party Lifecycle

### 12.1 Auto-creation on party creation

The `CreateParty` use case (or an event handler/port) must ensure a wallet row exists immediately after the party is persisted. This avoids every subsequent wallet operation having to lazily create the row.

Recommended approach:

- Add a `WalletRepository::create(wallet: &PlatformWallet)` method.
- After `PartyRepository::create` succeeds, call `CreateWallet::new(wallet_repo).execute(party_id).await`.
- Alternatively, implement `CreateParty` so it receives both `PartyRepository` and `WalletRepository` and performs both inserts in the same use case.

### 12.2 Soft-delete / reactivation

When a party is soft-deleted:

- Set `platform_wallets.is_active = false`.
- Block any future debits or credits.

When a party is reactivated:

- Set `platform_wallets.is_active = true`.

---

## 13. Integration with Deal Lifecycle

The wallet container is the settlement side of the deal state machine. Every deal-driven wallet mutation is recorded with the deal's `deal_id`, so each deal has a complete, auditable sub-ledger (per-deal sub-wallet) for every participating party.

| Deal transition | Wallet action | Transaction type | Per-deal effect |
|---|---|---|---|
| `TERMS_LOCKED → COMMITTED` | Consumer `balance` is debited and moved to `escrow_balance` for the deal value (+ platform fee) | `ESCROW_HOLD` | Consumer `contributed` and `heldInEscrow` increase |
| Milestone verified | Escrow is released to supplier/enhancer balances | `ESCROW_RELEASE` | Supplier/enhancer `released` increases; consumer `heldInEscrow` decreases |
| Deal cancelled | Escrow refunded to consumer (or split per cancellation terms) | `ADJUSTMENT` / `ESCROW_RELEASE` | Consumer `heldInEscrow` decreases; refund credited |
| Platform fee collection | Fee deducted from escrow or consumer balance | `FEE` | Consumer `feesPaid` increases |
| Deal completed | Final releases and fee settlement | `ESCROW_RELEASE`, `FEE` | All parties' per-deal positions finalised |

All deal-driven wallet mutations must check that the acting party is a participant in the deal and that the deal is in the correct status. The per-deal wallet view (`GET /api/v1/deals/{dealId}/wallet`) is recomputed from these rows on demand.

---

## 14. Security & Audit Considerations

1. **No direct balance updates.** Balances can only change via recorded transactions.
2. **Atomicity.** Every balance mutation is wrapped in a SQL transaction together with the `Transaction` insert.
3. **Idempotency.** External references should be unique per `(deal_id, transaction_type)` to avoid double-crediting a deposit or double-debiting a withdrawal.
4. **Approval gates.** Value-moving transactions create `PENDING` rows and require approval before mutating balances.
5. **Admin audit.** Adjustments record the admin user ID in `transaction_approvals` (or an `actor_user_id` audit column) and a reason in `description`.
6. **Read isolation.** A party can only read its own wallet unless the caller has an admin scope.

---

## 15. Open Points & Future Extensions

- **Real payment provider bridge:** keep `payment_method`/`external_reference` but optionally settle via Stripe/Flutterwave instead of purely mirroring external settlement.
- **Withdrawal limits:** daily/weekly withdrawal caps per party or per verification tier.
- **Interest or yield on escrow:** future feature for long-running deals.
- **Multi-currency support:** the schema uses `currency TEXT` but the domain currently hard-codes `POINTS`.
- **Wallet statements:** generate downloadable statements from `transactions`.
- **Per-deal materialised cache:** if per-deal wallet queries become hot, add a `deal_wallet_balances` table maintained by the same transaction that writes `transactions`.
- **Notification emails:** notify parties when deposits arrive, withdrawals complete, or escrow is released.

---

## 16. Glossary

| Term | Meaning |
|---|---|
| **Wallet Container** | The single `platform_wallets` row per party. It holds aggregate balances and is the parent of all per-deal sub-wallets. |
| **Per-Deal Sub-Wallet** | A logical wallet derived from `transactions` for one `(party, deal)` pair. No separate table row is required. |
| **Points** | Platform-internal unit of value; transactions mirror external physical settlements. |
| **Escrow Balance** | Aggregate points held in reserve across all active deals until milestones are verified or deals are cancelled. |
| **Pending Balance** | Aggregate points tied to transactions awaiting multi-party approval. |
| **Transaction** | A point movement between wallets or between a wallet and the platform. |
| **Transaction Approval** | Approval by every party involved in a transaction before it is marked verified/complete. |
| **Per-Deal Wallet View** | A derived ledger showing one party's contributed, escrowed, released, fee, and net points for a single deal. |
| **Adjustment** | A manual or system-initiated correction to a wallet balance. |
