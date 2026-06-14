# Plan: Automatic Deal Timeout Handling

## 1. Goal

Add a self-contained background timeout worker that automatically advances deals through the lifecycle when they have remained in a transient state longer than allowed. Timeouts must be:

1. **Configurable from `.env`** — global defaults per `DealStatus`.
2. **Overridable per deal** — individual deals can supply their own timeout values for any state, because real-world deals vary in urgency.
3. **Observable and auditable** — every automatic transition is recorded in `deal_history`.
4. **Safe** — only active, non-terminal states are evaluated; terminal states are ignored.

This plan does **not** modify source code. It describes the files, migrations, configuration, and behavior to implement.

---

## 2. Scope

### In scope

- New `deal_timeouts` configuration block loaded from `.env` / environment.
- New `timeout_overrides` column on `deals` to store per-deal overrides.
- New application use case `ProcessDealTimeouts` that scans for due deals and applies the correct transition.
- New background worker spawned in `api/src/main.rs`.
- Updates to the `Deal` domain aggregate, `DealRepository` port, and Postgres repository to carry `timeout_overrides`.
- Migration for the schema changes.
- Tests: config resolution, fake-repo unit tests, and an integration test that back-dates rows to simulate expiry.

### Out of scope (future work)

- Email/in-app notifications when a deal times out.
- Automatic deletion of expired drafts (transition to `EXPIRED` is implemented; physical deletion is a separate retention concern).
- Party-group governance voting on timeout-driven transitions.
- External scheduler integration (e.g., cron, pg_cron). The MVP uses an in-process Tokio worker.

---

## 3. State-to-timeout mapping

The following table maps each active `DealStatus` to the transition applied when its timeout elapses. All mappings are valid according to the existing `Deal::can_transition` matrix.

| Current status | Timeout transition | Recommended default |
|---|---|---|
| `DRAFT` | `EXPIRED` | 7 days |
| `SUGGESTED` | `EXPIRED` | 14 days |
| `PENDING_REVIEW` | `EXPIRED` | 14 days |
| `NEGOTIATING` | `ON_HOLD` | 30 days |
| `AWAITING_PARTY` | `ON_HOLD` | 14 days |
| `TERMS_LOCKED` | `CANCELLED` | 14 days |
| `COMMITTED` | `EXECUTING` | 3 days |
| `ON_HOLD` | `CANCELLED` | 30 days |
| `DISPUTED` | `ON_HOLD` | 14 days |
| `COMPLETED`, `CANCELLED`, `EXPIRED` | — | terminal; ignored |

The `DISPUTED` timeout is intentionally conservative: after 14 days the deal freezes in `ON_HOLD` so platform staff can mediate, rather than being cancelled automatically.

---

## 4. Configuration design

### 4.1 Config crate structure

Add two nested structs to `crates/infrastructure/src/config.rs`:

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct DealTimeoutSettings {
    #[serde(default = "default_draft_timeout_seconds")]
    pub draft_seconds: i64,
    #[serde(default = "default_suggested_timeout_seconds")]
    pub suggested_seconds: i64,
    // ... etc for every state in the mapping table
}

#[derive(Debug, Deserialize, Clone)]
pub struct DealTimeoutWorkerSettings {
    #[serde(default = "default_timeout_worker_enabled")]
    pub enabled: bool,
    #[serde(default = "default_timeout_worker_interval_seconds")]
    pub interval_seconds: u64,
    #[serde(default = "default_timeout_worker_batch_size")]
    pub batch_size: usize,
}
```

Then attach them to `Settings`:

```rust
pub struct Settings {
    // ... existing fields ...
    #[serde(default)]
    pub deal_timeouts: DealTimeoutSettings,
    #[serde(default)]
    pub deal_timeout_worker: DealTimeoutWorkerSettings,
}
```

### 4.2 Environment variable names

Because the config loader uses prefix `APP` and separator `__`:

```text
APP_DEAL_TIMEOUTS__DRAFT_SECONDS=604800
APP_DEAL_TIMEOUTS__SUGGESTED_SECONDS=1209600
APP_DEAL_TIMEOUTS__PENDING_REVIEW_SECONDS=1209600
APP_DEAL_TIMEOUTS__NEGOTIATING_SECONDS=2592000
APP_DEAL_TIMEOUTS__AWAITING_PARTY_SECONDS=1209600
APP_DEAL_TIMEOUTS__TERMS_LOCKED_SECONDS=1209600
APP_DEAL_TIMEOUTS__COMMITTED_SECONDS=259200
APP_DEAL_TIMEOUTS__ON_HOLD_SECONDS=2592000
APP_DEAL_TIMEOUTS__DISPUTED_SECONDS=1209600

APP_DEAL_TIMEOUT_WORKER__ENABLED=true
APP_DEAL_TIMEOUT_WORKER__INTERVAL_SECONDS=300
APP_DEAL_TIMEOUT_WORKER__BATCH_SIZE=100
```

### 4.3 Default values

| Variable | Default | Rationale |
|---|---|---|
| `*_DRAFT_SECONDS` | `604800` (7 days) | Drafts should be finalized or discarded quickly. |
| `*_SUGGESTED_SECONDS` | `1209600` (14 days) | Gives invited parties time to review. |
| `*_PENDING_REVIEW_SECONDS` | `1209600` (14 days) | Formal review window. |
| `*_NEGOTIATING_SECONDS` | `2592000` (30 days) | Negotiations can be lengthy. |
| `*_AWAITING_PARTY_SECONDS` | `1209600` (14 days) | Waiting for a missing party response. |
| `*_TERMS_LOCKED_SECONDS` | `1209600` (14 days) | Lock should lead to commitment or release. |
| `*_COMMITTED_SECONDS` | `259200` (3 days) | Standard preparation period before execution. |
| `*_ON_HOLD_SECONDS` | `2592000` (30 days) | Grace period to resolve blockers. |
| `*_DISPUTED_SECONDS` | `1209600` (14 days) | Escalation freeze after two weeks. |
| `*_WORKER__ENABLED` | `true` | Worker runs by default. |
| `*_WORKER__INTERVAL_SECONDS` | `300` (5 minutes) | Frequent enough for business use, cheap to run. |
| `*_WORKER__BATCH_SIZE` | `100` | Limits per-tick load. |

### 4.4 Per-deal override format

Add a nullable JSONB column `timeout_overrides` to `deals`. Example shapes:

```json
{
  "DRAFT": 1209600,
  "NEGOTIATING": 5184000,
  "COMMITTED": null
}
```

Rules:

- Keys are uppercase `DealStatus::as_str()` values.
- Values are integers representing seconds.
- A value of `null` disables the timeout for that state on that deal.
- Absent keys fall back to the global `.env` default.
- Only positive values are accepted; `0` or negative values are treated as disabled.

The override is exposed through the API on deal creation/update as an optional `timeout_overrides` object. Admins can also set it when acting with `admin:deals` / `admin:*`.

---

## 5. Database changes

### 5.1 Migration

Create `migrations/20260614140000_add_deal_timeout_overrides.sql`:

```sql
-- Per-deal timeout overrides.
ALTER TABLE deals
    ADD COLUMN IF NOT EXISTS timeout_overrides JSONB;

-- Index to make timeout scanning fast.
CREATE INDEX IF NOT EXISTS idx_deals_status_entered_at
    ON deals(deal_status, current_state_entered_at)
    WHERE deal_status NOT IN ('COMPLETED', 'CANCELLED', 'EXPIRED');
```

The partial index covers the exact query pattern the worker uses.

### 5.2 Query pattern

The worker needs to find deals whose `current_state_entered_at + timeout < now()`. Because the timeout is not a fixed interval per row, the query is state-driven:

```sql
SELECT id, deal_status, current_state_entered_at, timeout_overrides
FROM deals
WHERE deal_status = $1
  AND current_state_entered_at < now() - make_interval(secs => $2)
ORDER BY current_state_entered_at
LIMIT $3
FOR UPDATE SKIP LOCKED;
```

`$2` is the resolved timeout in seconds for that state (after applying per-deal overrides). Per-deal overrides can be applied in Rust after fetching candidates, or the query can use a more complex JSONB expression. For simplicity and correctness, the plan uses a two-step approach:

1. Query all active deals of the target status that entered the state before a generous horizon (e.g., the longest possible timeout for that status, or the default if no override can be shorter).
2. In Rust, compute the per-deal effective timeout and filter out deals that are not yet due.

This keeps the SQL simple and avoids JSONB arithmetic in the database.

---

## 6. Domain changes

### 6.1 `Deal` aggregate

Add one field to `crates/domain/src/entities/deal.rs`:

```rust
pub timeout_overrides: Option<serde_json::Value>,
```

The `Deal::new` constructor initializes it to `None`.

### 6.2 `DealRepository` port

Add a method to `crates/domain/src/repositories/deal_repository.rs`:

```rust
async fn find_deals_by_status(
    &self,
    status: DealStatus,
    entered_before: OffsetDateTime,
    limit: i64,
) -> Result<Vec<Deal>, DomainError>;
```

This returns full `Deal` aggregates so the use case can inspect `timeout_overrides`. Existing `update` and `record_history` methods are reused for mutation.

---

## 7. Application changes

### 7.1 New module: `crates/application/src/deals/process_timeouts.rs`

Introduce a `ProcessDealTimeouts` use case:

```rust
pub struct ProcessDealTimeouts {
    deal_repo: Arc<dyn DealRepository>,
    timeout_config: DealTimeoutConfig,
}
```

`DealTimeoutConfig` is a plain struct (defined in `application/src/deals/timeout_config.rs`) built from `DealTimeoutSettings`. It exposes:

```rust
impl DealTimeoutConfig {
    pub fn from_settings(settings: &DealTimeoutSettings) -> Self;
    pub fn timeout_for(&self, status: DealStatus, overrides: Option<&Value>) -> Option<Duration>;
    pub fn transition_for(&self, status: DealStatus) -> Option<DealStatus>;
}
```

`timeout_for` returns `None` when the timeout is disabled for that state/deal.

The use case execution:

1. For each active `DealStatus` in the mapping table:
   a. Compute the maximum possible horizon (using the smallest configured timeout, including possible overrides if cheap; otherwise the default).
   b. Fetch candidate deals entered before that horizon.
   c. For each candidate, resolve the effective timeout.
   d. If now >= `current_state_entered_at + timeout`, apply the mapped transition.
2. Load the aggregate via `find_aggregate_by_id`.
3. Call `deal.transition(target_status)` to enforce domain rules.
4. Update `actual_start_date` / `actual_end_date` when transitioning to `EXECUTING` or `COMPLETED`.
5. Persist the deal with `deal_repo.update`.
6. Record history with `deal_repo.record_history`:
   - `event_type`: `DEAL_TIMEOUT_TRANSITION`
   - `actor_party_id`: `NULL`
   - `details`: JSON containing `from_status`, `to_status`, `timeout_seconds`, `triggered_at`.
7. Return a summary of processed IDs and skipped IDs.

The use case is **idempotent**: re-running it on the same row is a no-op because the status has changed and `current_state_entered_at` is reset.

### 7.2 Why not reuse `ExecuteTransition`

`ExecuteTransition` requires an authenticated actor and validates party membership. Timeout transitions are system-initiated and have no actor. A dedicated use case avoids shoehorning a synthetic system user into the membership model and keeps audit semantics clean.

### 7.3 Integration with validation

Some timeout transitions have preconditions:

- `COMMITTED -> EXECUTING` requires milestones to be present.
- `TERMS_LOCKED -> CANCELLED` has no extra preconditions.

`ProcessDealTimeouts` should respect the same preconditions as `ExecuteTransition`. For `COMMITTED -> EXECUTING`, it checks `MilestoneRepository::count_by_deal`. If preconditions fail, the deal is skipped and a warning is logged; the next worker tick retries it. A new `deal_history` row records the failure (`DEAL_TIMEOUT_BLOCKED`) so support can investigate.

---

## 8. API / runtime changes

### 8.1 Background worker

Add a new worker function in `crates/infrastructure/src/workers/deal_timeout_worker.rs`:

```rust
pub async fn run_deal_timeout_worker(
    process_timeouts: Arc<ProcessDealTimeouts>,
    interval: Duration,
    batch_size: usize,
)
```

Behavior:

- Sleep for `interval` between ticks.
- On each tick, call `process_timeouts.execute(batch_size).await`.
- Log summary metrics (candidates found, transitions applied, blocked, errors).
- Swallow non-fatal errors so one bad row does not crash the worker.
- Use `tokio::time::interval` for drift-resistant scheduling.

### 8.2 Wiring in `crates/api/src/main.rs`

After the email worker spawn, conditionally spawn the timeout worker:

```rust
if settings.deal_timeout_worker.enabled {
    let timeout_worker = Arc::new(ProcessDealTimeouts::new(
        deal_repo.clone(),
        DealTimeoutConfig::from_settings(&settings.deal_timeouts),
    ));
    tokio::spawn(run_deal_timeout_worker(
        timeout_worker,
        Duration::from_secs(settings.deal_timeout_worker.interval_seconds),
        settings.deal_timeout_worker.batch_size,
    ));
}
```

No shutdown signal is added; the worker terminates with the process, which matches the existing email worker behavior.

### 8.3 AppState

`ProcessDealTimeouts` does not need to be added to `AppState` because it is only used by the background worker, not by HTTP handlers.

---

## 9. API contract changes

### 9.1 Create / update deal DTOs

Add an optional field:

```json
{
  "timeout_overrides": {
    "DRAFT": 1209600,
    "COMMITTED": null
  }
}
```

Validation rules:

- Keys must be valid `DealStatus` strings.
- Values must be `null` or a positive integer.
- At most one override per state.

### 9.2 Response shape

The `DealResult` DTO includes the new field so callers can read back the effective overrides:

```json
{
  "timeout_overrides": { ... }
}
```

---

## 10. Observability

Every automatic transition produces structured logs:

```text
INFO deal_timeout_transition
    deal_id=...
    from_status=DRAFT
    to_status=EXPIRED
    timeout_seconds=604800
```

Warnings are emitted when a deal is due but blocked:

```text
WARN deal_timeout_blocked
    deal_id=...
    status=COMMITTED
    reason="milestones are required before executing"
```

A Prometheus-style counter or future metric can be added later; the log fields are chosen to make metric extraction easy.

---

## 11. Testing plan

### 11.1 Unit tests

- `DealTimeoutConfig::timeout_for` respects defaults, overrides, disabled states, and invalid values.
- `DealTimeoutConfig::transition_for` returns the correct target for every active status and `None` for terminal statuses.

### 11.2 Application tests with fake repositories

- A deal in `DRAFT` for longer than the configured timeout is transitioned to `EXPIRED`.
- A deal with a per-deal override uses the override value.
- A deal with a disabled timeout (`null`) is not transitioned.
- A `COMMITTED` deal without milestones is skipped and logged.
- Re-running the use case on an already-transitioned deal is a no-op.

### 11.3 Integration tests

- Insert a `deals` row with `current_state_entered_at` in the past and `deal_status = 'DRAFT'`.
- Run `ProcessDealTimeouts`.
- Assert the row now has `deal_status = 'EXPIRED'` and a `deal_history` row exists.

### 11.4 Regression tests

- Run full `cargo test --workspace`.
- Run `cargo sqlx prepare --workspace`.
- Run `cargo llvm-cov --workspace --summary-only` and verify coverage remains > 80%.

---

## 12. Migration order

1. `migrations/20260614140000_add_deal_timeout_overrides.sql`
2. Run `sqlx migrate run`.
3. Run `cargo sqlx prepare --workspace` to regenerate `.sqlx/` metadata.

No existing data needs backfilling; `timeout_overrides` is nullable and all existing deals will simply use the global defaults.

---

## 13. Deployment / operational notes

- The worker is enabled by default. Set `APP_DEAL_TIMEOUT_WORKER__ENABLED=false` to disable in environments where an external scheduler will drive timeouts.
- In production, `APP_DEAL_TIMEOUT_WORKER__INTERVAL_SECONDS` can be raised to reduce load; the default 5 minutes is suitable for most cases.
- Because the worker uses `FOR UPDATE SKIP LOCKED`, multiple API instances can run the worker concurrently without double-processing the same row.
- The partial index keeps the scan cheap even as the `deals` table grows.

---

## 14. Files to modify

| File | Change |
|---|---|
| `crates/infrastructure/src/config.rs` | Add `DealTimeoutSettings`, `DealTimeoutWorkerSettings`, and attach to `Settings`. |
| `.env.example` | Add timeout-related environment variables with defaults. |
| `migrations/20260614140000_add_deal_timeout_overrides.sql` | Add `timeout_overrides` column and partial index. |
| `crates/domain/src/entities/deal.rs` | Add `timeout_overrides` field; update constructor and tests. |
| `crates/domain/src/repositories/deal_repository.rs` | Add `find_deals_by_status` port method. |
| `crates/infrastructure/src/repositories/postgres_deal_repository.rs` | Select and persist `timeout_overrides`; implement `find_deals_by_status`. |
| `crates/application/src/deals/mod.rs` | Export new modules. |
| `crates/application/src/deals/timeout_config.rs` | New: config resolution helpers. |
| `crates/application/src/deals/process_timeouts.rs` | New: `ProcessDealTimeouts` use case. |
| `crates/application/src/deals/dto.rs` | Add `timeout_overrides` to commands/results. |
| `crates/application/src/deals/tests.rs` | Add timeout tests. |
| `crates/application/src/test_helpers.rs` | Update fake `DealRepository` to support new method. |
| `crates/infrastructure/src/workers/deal_timeout_worker.rs` | New background worker. |
| `crates/infrastructure/src/workers/mod.rs` | Export worker. |
| `crates/api/src/main.rs` | Spawn worker when enabled. |
| `crates/api/src/dto/deals.rs` | Add `timeout_overrides` to request/response DTOs. |
| `crates/api/src/handlers/deals/create_deal.rs` | Accept `timeout_overrides`. |
| `crates/api/src/handlers/deals/update_deal.rs` | Accept `timeout_overrides`. |
| `crates/api/tests/create_user.rs` or new integration test file | API/DB timeout tests. |

---

## 15. Future extensions

- **Notifications**: emit a `DealTimedOut` domain event and enqueue emails to party members.
- **Draft cleanup**: a second worker that hard-deletes `EXPIRED` drafts after a retention period.
- **Admin override UI**: allow deal managers to pause/resume timeouts for a specific deal via `admin:deals`.
- **External scheduler**: expose `ProcessDealTimeouts` as a CLI command or HTTP admin endpoint so a Kubernetes CronJob can run it instead of the in-process worker.
