# Transaction Approval Workflow — Design Plan

> **Scope:** Define how every value-moving `Transaction` is approved by the involved parties before the platform ledger is mutated. This plan is derived from the existing Rust source code, `trust-score.md`, `deal-plan.md`, `wallets.md`, `hayaland-deal-plan.md`, and `3partydeal.pdf`.
>
> **Status:** Design only — no source code changes are described as already applied.

---

## 1. Current State

The wallet container (`PlatformWallet`) and the ledger table (`transactions`) already exist. The current code path in `PostgresWalletRepository::record_transaction` performs two things in one SQL transaction:

1. Updates the wallet container balances (`balance`, `escrow_balance`, `total_deposited`, `total_withdrawn`).
2. Inserts the `Transaction` row.

The `Transaction` entity has approval-related fields but they are not used by the current use cases:

```rust
pub struct Transaction {
    pub status: TransactionStatus,          // PENDING | VERIFIED | COMPLETE | REJECTED
    pub requires_approval: bool,            // currently always false
    pub approvals_required: i32,            // currently always 0
    pub approvals_received: i32,            // currently always 0
    // ...
}
```

The `Transaction::new` constructor defaults `requires_approval` to `false` and `status` to the value passed by the caller. All existing payment use cases (`DepositPoints`, `WithdrawPoints`, `HoldEscrow`, `ReleaseEscrow`, `DeductFee`, `RecordAdjustment`) create `VERIFIED` transactions and mutate balances immediately.

The `transaction_approvals` table is already in the migration set:

```sql
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

What is missing:
- A `TransactionApproval` domain entity.
- Repository methods to record pending transactions, record approvals, and finalise (or reject) transactions atomically.
- Application use cases for `ApproveTransaction` / `RejectTransaction` and `ListPendingTransactionApprovals`.
- API routes for parties to list, approve, and reject pending transactions.
- Domain methods on `PlatformWallet` to hold, release, and return `pending_balance`.

---

## 2. Goals

1. Every transaction that moves value between **distinct parties** must be approved by every involved party before balances change.
2. Self-only transactions (e.g., a party deposits into or withdraws from its own wallet) may remain auto-verified for the MVP.
3. `EscrowRelease` transactions require approval from **all three deal parties**, per the escrow rules in `3partydeal.pdf`.
4. Rejecting a transaction must never mutate balances; any held `pending_balance` is returned to the source.
5. The workflow must be atomic, auditable, and testable with the existing hexagonal architecture.

---

## 3. Approval Rules

Based on `trust-score.md` §8.5 and the escrow rules in `3partydeal.pdf` §4.3:

| Transaction type | Default `requires_approval` | Who must approve |
|---|---|---|
| `DEPOSIT` (to own wallet) | `false` | N/A — auto-verified |
| `WITHDRAWAL` (from own wallet) | `false` | N/A — auto-verified |
| `ESCROW_HOLD` (own balance → own escrow) | `false` | N/A — auto-verified after deal commitment |
| `ESCROW_RELEASE` | `true` | **All three deal parties** (not just `from_party_id` / `to_party_id`) |
| `FEE` | `true` | The party debited (`from_party_id`) |
| `ADJUSTMENT` | `true` | Any party whose wallet is debited or credited |

Rules for the approval process:

1. `approvals_required` is set when the transaction is created to the number of **distinct parties** that must approve.
2. Any active, non-observer member of an involved party may submit an approval on behalf of that party.
3. A party can submit only one decision per transaction (`UNIQUE (transaction_id, party_id)`).
4. Decision is either `APPROVED` or `REJECTED`.
5. If any required party rejects, the transaction moves to `REJECTED` and no ledger mutation occurs.
6. When all required parties approve, the transaction moves to `VERIFIED` (and optionally `COMPLETE`) and the ledger mutation is applied atomically.

---

## 4. Transaction Status Lifecycle

```text
PENDING → (all required parties approve) → VERIFIED → COMPLETE
     ↓ (any required party rejects)
  REJECTED
```

| Status | Meaning |
|---|---|
| `PENDING` | Awaiting required approvals; funds are held in `pending_balance` if the transaction debits a wallet. |
| `VERIFIED` | All required approvals received; the ledger mutation has been applied. |
| `COMPLETE` | External settlement reconciled (optional future state). |
| `REJECTED` | At least one required party rejected; no points moved. |

---

## 5. Domain Additions

### 5.1 `TransactionApproval` entity

```rust
pub struct TransactionApproval {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub party_id: Uuid,
    pub approved_by_user_id: Uuid,
    pub decision: ApprovalDecision,
    pub comment: Option<String>,
    pub created_at: OffsetDateTime,
}

pub enum ApprovalDecision {
    Approved,
    Rejected,
}
```

### 5.2 `ApprovalDecision` helpers

- `as_str()` returns `"APPROVED"` / `"REJECTED"`.
- `TryFrom<&str>` for repository mapping.

### 5.3 `Transaction` factory changes

Add a constructor that creates a pending transaction ready for approval:

```rust
impl Transaction {
    pub fn new_pending(
        id: Uuid,
        deal_id: Uuid,
        transaction_type: TransactionType,
        from_party_id: Option<Uuid>,
        to_party_id: Option<Uuid>,
        approvals_required: i32,
        amount: Decimal,
        description: Option<String>,
        payment_method: Option<String>,
        external_reference: Option<String>,
    ) -> Self;
}
```

This constructor sets:
- `status = TransactionStatus::Pending`
- `requires_approval = true`
- `approvals_required = approvals_required`
- `approvals_received = 0`
- `executed_at = None`

### 5.4 `PlatformWallet` pending-balance methods

The wallet container already has a `pending_balance` field. Add domain invariants:

```rust
impl PlatformWallet {
    /// Move available balance into pending while awaiting approval.
    pub fn hold_pending(&mut self, amount: Decimal) -> Result<(), DomainError>;

    /// Release pending balance back to available (on rejection).
    pub fn release_pending(&mut self, amount: Decimal) -> Result<(), DomainError>;

    /// Commit pending balance to escrow (e.g. approved escrow hold).
    pub fn commit_pending_to_escrow(&mut self, amount: Decimal) -> Result<(), DomainError>;

    /// Commit pending balance to available balance (e.g. approved deposit/release).
    pub fn commit_pending_to_balance(&mut self, amount: Decimal) -> Result<(), DomainError>;
}
```

For transactions that debit `escrow_balance` (e.g., cross-party escrow release), equivalent escrow-pending methods may be needed.

---

## 6. Repository Port Extensions

The current `WalletRepository` already owns transaction persistence. Extend it with approval-aware methods:

```rust
#[async_trait]
pub trait WalletRepository: Send + Sync {
    // existing methods: create, find_by_party_id, update, record_transaction,
    //                   find_transactions, count_transactions, compute_deal_wallet

    /// Persist a pending transaction without mutating wallet balances.
    async fn record_pending_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<(), DomainError>;

    /// Find a transaction by ID.
    async fn find_transaction_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Transaction>, DomainError>;

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
        wallet_mutations: &[(Uuid, PlatformWallet)], // party_id -> updated wallet
    ) -> Result<(), DomainError>;

    /// List pending transactions where the given party is an involved approver.
    async fn find_pending_transactions_for_party(
        &self,
        party_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Transaction>, DomainError>;

    /// Count pending transactions where the given party is an involved approver.
    async fn count_pending_transactions_for_party(
        &self,
        party_id: Uuid,
    ) -> Result<i64, DomainError>;
}
```

Implementation notes for `PostgresWalletRepository`:
- `record_pending_transaction` inserts the `Transaction` row with `status = 'PENDING'` and does **not** touch `platform_wallets`.
- `record_approval_and_finalise` runs inside a SQL transaction:
  1. Insert the `transaction_approvals` row.
  2. Update `transactions.approvals_received`.
  3. If rejected, set `status = 'REJECTED'` and reverse any `pending_balance` hold.
  4. If `approvals_received == approvals_required`, set `status = 'VERIFIED'`, update `executed_at`, and apply all wallet mutations.
- `find_pending_transactions_for_party` selects transactions where `status = 'PENDING'`, `requires_approval = true`, and the party appears in the involved set, excluding rows where the party already has an approval record.

---

## 7. Application Use Cases

### 7.1 `RecordTransaction`

A thin internal orchestrator that the existing payment use cases (`DepositPoints`, `WithdrawPoints`, `HoldEscrow`, `ReleaseEscrow`, `DeductFee`, `RecordAdjustment`) delegate to.

Responsibilities:
1. Decide `requires_approval` and `approvals_required` based on the transaction type and involved parties.
2. If auto-verified, apply the wallet mutation immediately (current behaviour).
3. If approval required:
   - Hold the source funds in `pending_balance`.
   - Persist the transaction as `PENDING`.
   - Do not mutate final balances yet.

### 7.2 `ApproveTransaction`

Command:

```rust
pub struct ApproveTransactionCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub transaction_id: Uuid,
    pub decision: ApprovalDecision,
    pub comment: Option<String>,
}
```

Steps:
1. Load the transaction by ID; reject if not found or not pending.
2. Verify the actor is an active, non-observer member of `actor_party_id`.
3. Verify `actor_party_id` is one of the required approvers for the transaction.
4. Verify the party has not already approved/rejected the transaction.
5. Record the approval.
6. If rejected:
   - Mark transaction `REJECTED`.
   - Return any held `pending_balance` to the source wallet(s).
7. If approved and final:
   - Mark transaction `VERIFIED`.
   - Apply the wallet mutations atomically.

Return `TransactionResult`.

### 7.3 `RejectTransaction`

Can be implemented as a thin wrapper around `ApproveTransaction` with `decision = Rejected`, or as a separate use case for clarity.

### 7.4 `ListPendingTransactionApprovals`

Command/query:

```rust
pub struct ListPendingApprovalsQuery {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
```

Returns pending transactions where `actor_party_id` is a required approver and has not yet voted, plus the list of parties that still need to vote.

### 7.5 `GetTransaction`

Optional helper returning a transaction and its current approvals.

---

## 8. API Routes

All routes require JWT authentication. Routes that operate on a specific party also require the `X-Party-ID` header, consistent with existing deal/party endpoints.

| Method | Route | Purpose |
|---|---|---|
| `GET` | `/api/v1/payments/transactions/pending-approvals` | List transactions awaiting approval by the acting party |
| `GET` | `/api/v1/payments/transactions/{id}` | Get a transaction and its approvals |
| `POST` | `/api/v1/payments/transactions/{id}/approve` | Approve a pending transaction |
| `POST` | `/api/v1/payments/transactions/{id}/reject` | Reject a pending transaction |

Request body for approve/reject:

```json
{
  "comment": "Looks correct"
}
```

Response returns the updated `TransactionResult` with current status and approval count.

---

## 9. Integration with Existing Payment Use Cases

The existing payment use cases can be evolved without breaking their public DTOs:

- `DepositPoints`, `WithdrawPoints`, `HoldEscrow`, `DeductFee`, `RecordAdjustment` continue to accept the same commands.
- Internally they call `RecordTransaction`, which decides whether the resulting transaction is auto-verified or pending.
- For the MVP, only self-only operations remain auto-verified.
- `ReleaseEscrow` should be extended to support cross-party releases:
  - If `to_party_id == actor_party_id`, auto-verified (self-release).
  - If `to_party_id` is another deal party, create a pending `ESCROW_RELEASE` transaction requiring approval from **all three deal parties**.

This keeps the user experience simple for single-party operations while enforcing multi-party approval wherever value moves between parties.

---

## 10. Escrow Release Special Case

Per `3partydeal.pdf` §4.3:

> Release requires: milestone ACCEPTED + 24h dispute window clear

And:

> Platform holds funds; releases on milestone verification. 3-party approval for release.

Therefore the release workflow is:

1. A milestone is verified (future milestone feature).
2. The platform creates an `ESCROW_RELEASE` transaction for the appropriate amount and recipient.
3. The transaction is `PENDING` and requires approval from all three deal parties.
4. Once all three approve, the recipient's wallet balance is credited and the source party's escrow balance is debited.
5. If any party rejects, the release is cancelled and the funds remain in escrow.

Until the milestone feature is built, the admin/system can create release transactions manually via `ReleaseEscrow`.

---

## 11. Security & Audit

1. **No direct balance updates** — balances change only through approved transactions.
2. **Atomicity** — approval inserts and ledger mutations happen in one SQL transaction.
3. **Idempotency** — duplicate approval attempts from the same party return the existing decision or a conflict error.
4. **Access control** — only members of an involved party may approve/reject; only that party's own members may view its pending approvals list.
5. **Admin override** — platform users with the `admin` role or the `admin:transactions` / `admin:*` scope may view and manage any transaction. Admins still provide an `X-Party-ID` header to identify the party on whose behalf they act; the membership check is skipped, but the party must still be one of the transaction's required approvers when submitting a decision. This supports support, moderation, and dispute-resolution workflows.
6. **Audit trail** — `transaction_approvals` records `approved_by_user_id`, decision, comment, and timestamp.
7. **Rejection rollback** — held `pending_balance` is always returned to the source wallet on rejection.

---

## 12. Testing Strategy

- **Domain tests**: `Transaction::new_pending`, `PlatformWallet` pending-balance methods, `ApprovalDecision` round-trips.
- **Application tests** with `FakeWalletRepo`:
  - Auto-verified self-deposit does not require approval.
  - Pending transaction holds `pending_balance`.
  - Approval by all involved parties mutates balances and sets `VERIFIED`.
  - Rejection by one party sets `REJECTED` and returns `pending_balance`.
  - A party cannot vote twice.
  - A non-involved party cannot vote.
  - Listing pending approvals only returns transactions awaiting the actor's party.
- **Postgres integration tests**:
  - `record_pending_transaction` leaves balances unchanged.
  - `record_approval_and_finalise` updates counts/status/balances atomically.
  - `find_pending_transactions_for_party` respects the party filter and excludes already-voted rows.
- **API tests**:
  - Approve/reject endpoints return correct status codes.
  - Missing `X-Party-ID` returns 400.
  - Non-member user returns 403.

Target: maintain >80% line coverage and keep `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo sqlx prepare --workspace --check` green.

---

## 13. Definition of Done

- [ ] `TransactionApproval` domain entity exists.
- [ ] `PlatformWallet` supports `pending_balance` hold/release/commit operations.
- [ ] `WalletRepository` exposes pending-transaction and approval methods.
- [ ] `PostgresWalletRepository` implements them atomically.
- [ ] `ApproveTransaction`, `RejectTransaction`, and `ListPendingTransactionApprovals` use cases exist and are unit-tested.
- [ ] API routes for listing, approving, and rejecting pending transactions are wired.
- [ ] Existing payment use cases route through `RecordTransaction` and use the approval rules in §3.
- [ ] `cargo test` passes.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy -- -D warnings` passes.
- [ ] `cargo sqlx prepare --workspace --check` passes.
- [ ] `cargo llvm-cov --workspace` reports >80% line coverage.

---

## 14. Open Questions / Future Extensions

- Should `WITHDRAWAL` require approval by a second party admin inside the same party (dual control)?
- Should the platform support delegated representatives (one user can approve for a party) or multi-entity voting patterns (`ANY_ONE`, `MAJORITY`, `ALL`, `THRESHOLD`, `WEIGHTED`) from `3partydeal.pdf` §4.7?
- Should `COMPLETE` status be set automatically when a transaction is verified, or only after external reconciliation?
- How should the system handle an approver changing their decision before finalisation? (Current design: one decision per party, immutable.)
