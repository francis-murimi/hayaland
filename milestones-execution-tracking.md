# Milestones & Execution Tracking — Design Plan

> **Scope:** Define how deal milestones are created, tracked, completed, verified, and how verified milestones trigger escrow releases through the transaction approval workflow.
>
> **Sources:** existing Rust source code, `migrations/20260613013000_create_terms_value_distributions_milestones.sql`, `trust-score.md`, `wallets.md`, `transaction-approval-workflow.md`, `deal-plan.md`, `hayaland-deal-plan.md`, and `3partydeal.pdf`.
>
> **Status:** Design only — no source code changes are described as already applied.

---

## 1. Current State

The `milestones` table already exists in the migration set:

```sql
CREATE TABLE IF NOT EXISTS milestones (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    milestone_name TEXT NOT NULL,
    description TEXT,
    assigned_to_party_id UUID REFERENCES parties(id),
    due_date DATE,
    completion_criteria TEXT NOT NULL,
    milestone_status TEXT NOT NULL DEFAULT 'PENDING'
        CHECK (milestone_status IN ('PENDING','IN_PROGRESS','COMPLETED','VERIFIED','MISSED')),
    completion_percentage DECIMAL NOT NULL DEFAULT 0,
    payment_trigger_amount DECIMAL, -- in platform points
    completed_at TIMESTAMPTZ,
    verified_by_party_id UUID REFERENCES parties(id),
    display_order INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

There is **no** domain `Milestone` entity, no `MilestoneRepository`, and no milestone use cases yet. The deal state machine already knows `Committed → Executing` and `Executing → Completed`, but `ExecuteTransition` does not yet implement those transitions.

The transaction approval workflow (described in `transaction-approval-workflow.md`) is the natural companion to milestones: when a milestone is verified, the platform creates a pending `ESCROW_RELEASE` transaction, and that transaction must be approved by all three deal parties before points move.

---

## 2. Goals

1. Allow deal participants to define milestones while a deal is in `COMMITTED` or `EXECUTING` status.
2. Track milestone lifecycle: `PENDING → IN_PROGRESS → COMPLETED → VERIFIED`.
3. Support overdue detection (`MISSED`) for milestones that pass their due date without being completed.
4. On verification, optionally create a pending `ESCROW_RELEASE` transaction for the milestone's `payment_trigger_amount`.
5. Ensure every release goes through the transaction approval workflow (3-party approval).
6. Enable deal-level progress reporting and automatic transition to `COMPLETED` when all milestones are verified.

---

## 3. Milestone Status Lifecycle

Use the statuses already in the migration:

```text
PENDING → IN_PROGRESS → COMPLETED → VERIFIED
  │           │
  └───────────┴──→ MISSED (if due date passes without completion)
```

| Status | Meaning |
|---|---|
| `PENDING` | Milestone defined but work has not started. |
| `IN_PROGRESS` | Assigned party has started work. |
| `COMPLETED` | Assigned party has submitted deliverables and marked it complete. |
| `VERIFIED` | Verifier has reviewed and accepted the completion evidence. |
| `MISSED` | Due date passed without completion (set by a background/overdue check). |

Rules:
- Only the `assigned_to_party_id` may move a milestone to `IN_PROGRESS` or `COMPLETED`.
- Only the `verified_by_party_id` may move a milestone from `COMPLETED` to `VERIFIED`.
- A milestone cannot be modified after it is `VERIFIED`.
- Milestones can only be created, updated, or deleted while the deal is `COMMITTED` or `EXECUTING`.

---

## 4. Domain Additions

### 4.1 `MilestoneStatus`

```rust
pub enum MilestoneStatus {
    Pending,
    InProgress,
    Completed,
    Verified,
    Missed,
}
```

- `as_str()` returns `"PENDING"`, `"IN_PROGRESS"`, `"COMPLETED"`, `"VERIFIED"`, `"MISSED"`.
- `TryFrom<&str>` for repository mapping.

### 4.2 `Milestone` entity

```rust
pub struct Milestone {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub milestone_name: String,        // validated 3–200 chars
    pub description: Option<String>,
    pub assigned_to_party_id: Uuid,
    pub due_date: Option<time::Date>,
    pub completion_criteria: String,   // validated non-empty
    pub milestone_status: MilestoneStatus,
    pub completion_percentage: rust_decimal::Decimal, // 0–100
    pub payment_trigger_amount: Option<rust_decimal::Decimal>,
    pub completed_at: Option<OffsetDateTime>,
    pub verified_by_party_id: Uuid,
    pub display_order: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

Domain invariants:
- `milestone_name` is 3–200 characters.
- `completion_criteria` is non-empty.
- `completion_percentage` is between 0 and 100.
- `payment_trigger_amount`, if present, is positive.
- `verified_by_party_id` must be a different party than `assigned_to_party_id`.
- Both `assigned_to_party_id` and `verified_by_party_id` must be participants in the deal.

### 4.3 Optional: `VerificationMethod`

`3partydeal.pdf` describes verification methods such as `PHOTO_EVIDENCE`, `DOCUMENT_REVIEW`, `THIRD_PARTY_INSPECTION`, `IoT_SENSOR_DATA`, `COUNTER_PARTY_ACCEPT`, and `PLATFORM_AUTO_VERIFY`. The existing schema does not have a dedicated column for this.

Options:
- **MVP:** include the method as text inside `completion_criteria` or `description`.
- **Later:** add a `verification_method TEXT` column to `milestones` via a new migration.

This plan recommends the MVP option to avoid a new migration.

---

## 5. Repository Port

Create a new `MilestoneRepository` port in `crates/domain/src/repositories/`:

```rust
#[async_trait]
pub trait MilestoneRepository: Send + Sync {
    async fn create(&self, milestone: &Milestone) -> Result<(), DomainError>;

    async fn update(&self, milestone: &Milestone) -> Result<(), DomainError>;

    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Milestone>, DomainError>;

    async fn find_by_deal(
        &self,
        deal_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Milestone>, DomainError>;

    async fn count_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError>;

    async fn count_verified_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError>;
}
```

Infrastructure:
- `PostgresMilestoneRepository` in `crates/infrastructure/src/repositories/postgres_milestone_repository.rs`.
- Uses `sqlx::query!` against the existing `milestones` table.
- Run `cargo sqlx prepare --workspace` after adding queries.

---

## 6. Application Use Cases

### 6.1 `CreateMilestone`

Command:

```rust
pub struct CreateMilestoneCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub milestone_name: String,
    pub description: Option<String>,
    pub assigned_to_party_id: Uuid,
    pub verified_by_party_id: Uuid,
    pub due_date: Option<time::Date>,
    pub completion_criteria: String,
    pub payment_trigger_amount: Option<Decimal>,
    pub display_order: i32,
}
```

Rules:
- Actor must be a member of a participating party.
- Deal must be `COMMITTED` or `EXECUTING`.
- `assigned_to_party_id` and `verified_by_party_id` must be distinct deal participants.

### 6.2 `UpdateMilestone`

Allows updating name, description, due date, completion criteria, payment trigger, verifier, and display order. Allowed only while the milestone is not `VERIFIED` and the deal is `COMMITTED` or `EXECUTING`.

### 6.3 `StartMilestone`

Moves status from `PENDING` to `IN_PROGRESS`. Only the assigned party may call it.

### 6.4 `CompleteMilestone`

Moves status from `IN_PROGRESS` to `COMPLETED` and records `completed_at`. Only the assigned party may call it. Optionally accepts a deliverables/evidence text.

### 6.5 `VerifyMilestone`

Moves status from `COMPLETED` to `VERIFIED` and records `verified_by_party_id`. Only the verifier party may call it.

If `payment_trigger_amount` is set:

1. Create a pending `ESCROW_RELEASE` transaction:
   - `deal_id`: the milestone's deal.
   - `from_party_id`: the party whose escrow funds the release (typically the consumer; determined from the deal's `ValueDistribution` or from the existing escrow holder).
   - `to_party_id`: `milestone.assigned_to_party_id`.
   - `amount`: `payment_trigger_amount`.
   - `milestone_id`: the milestone ID.
   - `status`: `PENDING`.
   - `requires_approval`: `true`.
   - `approvals_required`: 3 (all deal parties).
2. Persist the transaction through the transaction approval workflow's `record_pending_transaction` path.

After verification, if **all** milestones for the deal are now `VERIFIED`, the use case may optionally trigger the `Executing → Completed` deal transition. To keep responsibilities clean, the recommended approach is:
- `VerifyMilestone` records that the last milestone is verified.
- A subsequent call to `ExecuteTransition` (deal state machine) transitions the deal to `COMPLETED` when all milestones are verified.

### 6.6 `ListMilestones`

Lists milestones for a deal, ordered by `display_order`, for any deal participant.

### 6.7 `GetDealProgress`

Returns:

```rust
pub struct DealProgressResult {
    pub deal_id: Uuid,
    pub total_milestones: i64,
    pub verified_milestones: i64,
    pub completed_milestones: i64,
    pub in_progress_milestones: i64,
    pub missed_milestones: i64,
    pub overall_completion_percentage: Decimal,
}
```

`overall_completion_percentage` is the average of `completion_percentage` across all milestones, or `100` when all are `VERIFIED`.

---

## 7. API Routes

All routes require JWT authentication and, where a party acts, the `X-Party-ID` header.

| Method | Route | Purpose |
|---|---|---|
| `POST` | `/api/v1/deals/{id}/milestones` | Create a milestone |
| `GET` | `/api/v1/deals/{id}/milestones` | List milestones |
| `PATCH` | `/api/v1/deals/{id}/milestones/{milestoneId}` | Update a milestone |
| `DELETE` | `/api/v1/deals/{id}/milestones/{milestoneId}` | Delete a milestone (only before `VERIFIED`) |
| `POST` | `/api/v1/deals/{id}/milestones/{milestoneId}/start` | Mark `IN_PROGRESS` |
| `POST` | `/api/v1/deals/{id}/milestones/{milestoneId}/complete` | Mark `COMPLETED` |
| `POST` | `/api/v1/deals/{id}/milestones/{milestoneId}/verify` | Mark `VERIFIED`; triggers escrow release transaction |
| `GET` | `/api/v1/deals/{id}/progress` | Get deal progress summary |

`verify` request body (optional):

```json
{
  "comment": "Photos and invoice reviewed and accepted"
}
```

Response returns the updated milestone and, if a payment was triggered, the pending transaction ID.

---

## 8. Integration with Transaction Approval Workflow

This is the most important cross-cutting concern. Milestones are the **business trigger**; the transaction approval workflow is the **mechanism**.

### 8.1 Sequence: milestone verification → release

1. Verifier calls `POST /api/v1/deals/{id}/milestones/{mId}/verify`.
2. `VerifyMilestone` checks permissions and updates the milestone to `VERIFIED`.
3. If `payment_trigger_amount` is present, `VerifyMilestone` builds an `ESCROW_RELEASE` transaction and calls the transaction approval workflow's pending-transaction path.
4. The transaction is inserted with:
   - `status = PENDING`
   - `requires_approval = true`
   - `approvals_required = 3`
   - No wallet balances are mutated yet.
5. All three deal parties see the pending transaction in `GET /api/v1/payments/transactions/pending-approvals`.
6. Each party approves (or rejects) via the transaction approval endpoints.
7. Once all three approve, the transaction approval workflow:
   - Sets the transaction to `VERIFIED`.
   - Debits the source party's `escrow_balance`.
   - Credits the assigned party's `balance`.
8. If any party rejects, the transaction becomes `REJECTED` and no points move. The milestone remains `VERIFIED` (the work was accepted), but the payment must be re-initiated manually or by an admin adjustment.

### 8.2 Source of released funds

The `from_party_id` of the `ESCROW_RELEASE` transaction is the party that committed the escrow for the deal. In the current consumer-funded escrow model this is the **Consumer**. Future models (supplier bond, revenue share) may change the source; the milestone use case should determine the source from the deal's `ValueDistribution` or from a future `escrow_funder_party_id` field.

For the MVP, derive the source as the Consumer party in the deal's participations.

### 8.3 Platform fees on release

The deal already stores `platform_fee_percentage` and `platform_fee_amount`. When a milestone release is verified and approved, a separate `FEE` transaction can be created for the platform's portion. For the first milestone slice, this can be deferred and handled by a later settlement/reconciliation step to keep the two features decoupled.

---

## 9. Deal State Machine Additions

The existing `Deal::can_transition` already allows:

- `Committed → Executing`
- `Executing → Completed`

`ExecuteTransition` should be extended to support these transitions:

- `Committed → Executing`:
  - Allowed when the deal has at least one milestone and all parties have signalled ready, or after the 3-day prep period has elapsed.
  - Sets `actual_start_date`.
- `Executing → Completed`:
  - Allowed only when all milestones for the deal are `VERIFIED`.
  - Sets `actual_end_date`.
  - Optionally triggers review requests (future Reviews feature).

---

## 10. Security & Access Control

1. **Deal participation required** — only members of the three participating parties may view or mutate milestones for a deal.
2. **Role-specific actions** — only the assigned party may start/complete; only the verifier may verify.
3. **Status gating** — milestones are immutable once `VERIFIED`; edits are blocked after deal leaves `EXECUTING`.
4. **No direct balance impact** — `VerifyMilestone` never touches wallet balances directly; it only creates a pending transaction.
5. **Audit** — every milestone status change is recorded with `completed_at`, `verified_by_party_id`, and the transaction ID of any triggered release.

---

## 11. Testing Strategy

- **Domain tests**:
  - Milestone status transitions and invariant enforcement.
  - Invalid names, empty completion criteria, out-of-range percentages.
- **Application tests** with fake repositories:
  - Create/update/delete milestones.
  - Only assigned party can complete; only verifier can verify.
  - Verify triggers a pending `ESCROW_RELEASE` transaction with `approvals_required = 3`.
  - Listing and progress calculation.
- **Postgres integration tests**:
  - `PostgresMilestoneRepository` CRUD and filtering.
  - End-to-end: verify milestone → transaction appears pending → approvals release funds.
- **API tests**:
  - All routes return correct status codes.
  - Missing `X-Party-ID` and non-participants are rejected.

Target: maintain >80% line coverage and keep `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo sqlx prepare --workspace --check` green.

---

## 12. Definition of Done

- [ ] `Milestone` domain entity and `MilestoneStatus` exist.
- [ ] `MilestoneRepository` port and `PostgresMilestoneRepository` implementation exist.
- [ ] Use cases `CreateMilestone`, `UpdateMilestone`, `StartMilestone`, `CompleteMilestone`, `VerifyMilestone`, `ListMilestones`, `GetDealProgress` exist and are unit-tested.
- [ ] API routes for milestones and progress are wired.
- [ ] `VerifyMilestone` creates a pending `ESCROW_RELEASE` transaction through the transaction approval workflow.
- [ ] `ExecuteTransition` supports `Committed → Executing` and `Executing → Completed`.
- [ ] `cargo test` passes.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy -- -D warnings` passes.
- [ ] `cargo sqlx prepare --workspace --check` passes.
- [ ] `cargo llvm-cov --workspace` reports >80% line coverage.

---

## 13. Integration & Implementation Order

### Recommended order

1. **Implement the Transaction Approval Workflow first** (`transaction-approval-workflow.md`).
   - It is the smaller, more focused dependency.
   - It provides the reusable mechanism (`record_pending_transaction`, `record_approval_and_finalise`, pending approval list, approve/reject endpoints) that milestones need.
   - It can be unit-tested and API-tested in isolation using simple `DEPOSIT`/`WITHDRAWAL`/`ESCROW_HOLD`/`ESCROW_RELEASE` scenarios.

2. **Then implement Milestones & Execution Tracking** (`milestones-and-execution.md`).
   - Build on top of the existing deal aggregate and the newly approved transaction workflow.
   - The milestone work adds the business semantics (who delivers, who verifies, when funds are released).
   - The final integration test verifies the full flow: create deal → commit → create milestone → verify milestone → approve release transaction → wallet balances update.

### Why this order?

- Milestones without transaction approval would only be a task tracker; the release-of-funds step would be blocked or have to be rebuilt later.
- Transaction approval without milestones is still immediately useful for deposits, withdrawals, escrow holds, and admin adjustments that involve multiple parties.
- The transaction approval workflow defines the data model (`TransactionApproval`, pending balance handling, approval counts) that milestone releases depend on.

### Alternative (single combined milestone)

If product pressure demands both at once, implement them in the **same milestone but in sequence**:

- Week 1: transaction approval workflow (domain + application + API + tests).
- Week 2: milestones (domain + application + API) + integration with the approval workflow.

Either way, the transaction approval code must be stable before milestone verification starts creating pending release transactions.
