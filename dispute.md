# Dispute Handling — Implementation Design

> **Scope:** Design for a complete dispute-handling feature for the Hayaland 3-party deal platform.
>
> **Audience:** Backend engineers, QA engineers, product owners, and platform admins.
>
> **Based on:** `3partydeal.pdf` Software Design Document (§3.25, §4.5.5, §8.6.4), `hayaland-deal-plan.md`, `deal-plan.md`, `deal-timeout.md`, `trust-score.md`, `admin-dontrol.md`, `party-guide.md`, `AGENTS.md`, and the existing Rust codebase.
>
> **Status:** Design document. No source code is modified by this document.

---

## 1. Goals

1. **Give deal participants a formal conflict-resolution path.** Any party in a deal can raise a dispute when another party fails to perform, delivers poor quality, breaches terms, or commits fraud.
2. **Provide admins and scoped mediators with a queue and resolution tools.** Admins with `admin:disputes` (or `admin:*`) can review evidence, escalate, resolve, and decide the next deal status.
3. **Protect deal state and trust scores.** Raising a dispute moves the deal to `DISPUTED`; resolving it returns the deal to `EXECUTING`, moves it to `COMPLETED`, or cancels it. Trust scores are recalculated for all affected parties.
4. **Reuse existing architecture.** The feature follows the established hexagonal pattern (`domain` → `application` → `infrastructure` → `api`) and mirrors the `reviews` and `verifications` modules.
5. **Achieve >86% test coverage** through domain unit tests, application fake-repo tests, Postgres integration tests, and API tests.

---

## 2. Domain Model

### 2.1 Core entities

A dispute is a first-class aggregate. It is always scoped to a single `Deal` and raised by one of the deal's participating parties.

| Concept | Rust type | Description |
|---------|-----------|-------------|
| `Dispute` | `domain::entities::Dispute` | The dispute aggregate root. |
| `DisputeType` | enum | Classification of the conflict. |
| `DisputeStatus` | enum | Lifecycle state of the dispute. |
| `ResolutionType` | enum | How the dispute was resolved. |
| `ResolutionOutcome` | enum | Who the resolution favored. |
| `DisputeSeverity` | enum | Impact level used for trust-score penalties. |

### 2.2 `DisputeType`

Maps to the categories defined in `3partydeal.pdf` §3.25:

```rust
pub enum DisputeType {
    NonPayment,
    NonDelivery,
    QualityIssue,
    BreachOfTerms,
    Communication,
    ScopeDisagreement,
    DeliveryDelay,
    ForceMajeure,
    Fraud,
    Other,
}
```

Database representation: `TEXT` with a `CHECK` constraint.

### 2.3 `DisputeStatus`

```rust
pub enum DisputeStatus {
    Open,
    UnderReview,
    Mediation,
    Escalated,
    Resolved,
    Rejected,
}
```

State machine:

```text
OPEN ──► UNDER_REVIEW ──► RESOLVED
  │           │
  │           ▼
  │        ESCALATED ──► RESOLVED
  │
  ▼
REJECTED
```

Rules:
- `MEDIATION` and `ESCALATED` are both valid intermediate states. For the MVP they represent the same escalation tier; they can be split later without changing the public API.
- Once `RESOLVED` or `REJECTED`, the dispute is immutable except for `admin_notes`.

### 2.4 `ResolutionType`

```rust
pub enum ResolutionType {
    Amicable,   // Parties reached agreement themselves
    Mediated,   // Platform mediator imposed/facilitated a decision
    Arbitrated, // Binding arbitration
    Withdrawn,  // Complainant withdrew the dispute
}
```

### 2.5 `ResolutionOutcome`

```rust
pub enum ResolutionOutcome {
    InFavorOfRaised,   // Complainant's claim upheld
    InFavorOfAgainst,  // Respondent's defense upheld
    Split,             // Partial relief or shared blame
    Dismissed,         // Claim found frivolous or unsupported
}
```

### 2.6 `DisputeSeverity`

```rust
pub enum DisputeSeverity {
    Low,
    Medium,
    High,
}
```

Severity is chosen by the resolving admin/mediator and feeds into the trust-score penalty calculation. It is independent of `ResolutionType`.

### 2.7 `Dispute` struct

```rust
pub struct Dispute {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub raised_by_party_id: Uuid,
    pub raised_by_user_id: Uuid,
    pub against_party_id: Option<Uuid>,
    pub dispute_type: DisputeType,
    pub dispute_status: DisputeStatus,
    pub resolution_type: Option<ResolutionType>,
    pub resolution_outcome: Option<ResolutionOutcome>,
    pub severity: Option<DisputeSeverity>,
    pub description: String,
    pub evidence_urls: Vec<String>,
    pub admin_notes: Option<String>,
    pub resolution_notes: Option<String>,
    pub resolved_by_user_id: Option<Uuid>,
    pub resolved_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

### 2.8 `DisputeResponse` struct

```rust
pub struct DisputeResponse {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub party_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
    pub created_at: OffsetDateTime,
}
```

Factory:

```rust
impl DisputeResponse {
    pub fn new(
        id: Uuid,
        dispute_id: Uuid,
        party_id: Uuid,
        user_id: Uuid,
        message: String,
    ) -> Self;
}
```

Rules:
- Responses are immutable once posted.
- Any deal participant (or admin acting on behalf of a party) can post a response.
- Responses are returned when a single dispute is fetched, ordered by `created_at ASC`.

Factory and transition methods:

```rust
impl Dispute {
    pub fn new(
        id: Uuid,
        deal_id: Uuid,
        raised_by_party_id: Uuid,
        raised_by_user_id: Uuid,
        against_party_id: Option<Uuid>,
        dispute_type: DisputeType,
        description: String,
        evidence_urls: Vec<String>,
    ) -> Self;

    pub fn submit_for_review(&mut self) -> Result<(), DomainError>;
    pub fn escalate(&mut self) -> Result<(), DomainError>;
    pub fn resolve(
        &mut self,
        resolution_type: ResolutionType,
        resolution_outcome: ResolutionOutcome,
        severity: DisputeSeverity,
        resolution_notes: Option<String>,
        resolved_by_user_id: Uuid,
    ) -> Result<(), DomainError>;
    pub fn reject(
        &mut self,
        reason: String,
        resolved_by_user_id: Uuid,
    ) -> Result<(), DomainError>;
}
```

---

## 3. State Machines

### 3.1 Dispute state machine

```text
┌─────────┐      submit for review       ┌───────────────┐
│  OPEN   │ ───────────────────────────► │ UNDER_REVIEW  │
└────┬────┘                              └───────┬───────┘
     │                                            │
     │ reject                                     │ escalate
     ▼                                            ▼
┌─────────┐                              ┌───────────────┐
│ REJECTED│                              │  ESCALATED    │
└─────────┘                              │  / MEDIATION  │
                                         └───────┬───────┘
                                                 │
                                                 │ resolve
                                                 ▼
                                          ┌───────────────┐
                                          │   RESOLVED    │
                                          └───────────────┘
```

### 3.2 Deal status mapping

| Dispute event | Deal transition | Notes |
|---------------|-----------------|-------|
| Dispute raised | `EXECUTING → DISPUTED` or `ON_HOLD → DISPUTED` | Only active, non-terminal deals can be disputed. |
| Resolved, resume work | `DISPUTED → EXECUTING` | Default when the deal should continue. |
| Resolved, partial/no further work | `DISPUTED → COMPLETED` | Used when the resolution effectively finishes the deal. |
| Resolved, terminate | `DISPUTED → CANCELLED` | Used for breach, fraud, or mutual termination. |
| Unresolved timeout (14 days) | `DISPUTED → ON_HOLD` | Existing `ProcessDealTimeouts` worker handles this automatically. |

---

## 4. Authorization & Scopes

### 4.1 New scopes

| Scope | Who gets it | Purpose |
|-------|-------------|---------|
| `disputes:read` | `user` role | List/get disputes for deals the user participates in. |
| `disputes:write` | `user` role | Raise a dispute, submit evidence, respond. |
| `admin:disputes` | `admin` role | Full admin queue, escalation, resolution, and override. |
| `admin:*` | `admin` role | Super-admin fallback (existing pattern). |

Scope grants are seeded in the migration via `UPDATE role_definitions`.

### 4.2 Access rules

| Action | Required scope | Additional rules |
|--------|----------------|------------------|
| Raise dispute | `disputes:write` or `admin:disputes` / `admin:*` | Caller (or `X-Party-ID`) must be a deal participant. Deal must be `EXECUTING` or `ON_HOLD`. |
| List deal disputes | `disputes:read` or `admin:disputes` / `admin:*` | Caller must be a participant, unless admin. |
| Get dispute | `disputes:read` or `admin:disputes` / `admin:*` | Caller must be a participant, unless admin. |
| Submit evidence | `disputes:write` or `admin:disputes` / `admin:*` | Only the raising party or an admin. |
| Respond to dispute | `disputes:write` or `admin:disputes` / `admin:*` | Any deal participant or admin. |
| Escalate dispute | `admin:disputes` or `admin:*` | Admin only. |
| Resolve/reject dispute | `admin:disputes` or `admin:*` | Admin only. |
| Admin list disputes | `admin:disputes` or `admin:*` | Admin only. |

Admin override follows the existing `admin-dontrol.md` pattern: admins supply an `X-Party-ID` header to act on behalf of any participating party.

---

## 5. Database Schema

### 5.1 Migration

File: `migrations/20260615160000_create_disputes.sql`

```sql
CREATE TABLE IF NOT EXISTS disputes (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    raised_by_party_id UUID NOT NULL REFERENCES parties(id),
    raised_by_user_id UUID NOT NULL REFERENCES users(id),
    against_party_id UUID REFERENCES parties(id),
    dispute_type TEXT NOT NULL
        CHECK (dispute_type IN (
            'NON_PAYMENT','NON_DELIVERY','QUALITY_ISSUE','BREACH_OF_TERMS',
            'COMMUNICATION','SCOPE_DISAGREEMENT','DELIVERY_DELAY','FORCE_MAJEURE',
            'FRAUD','OTHER'
        )),
    dispute_status TEXT NOT NULL DEFAULT 'OPEN'
        CHECK (dispute_status IN ('OPEN','UNDER_REVIEW','MEDIATION','ESCALATED','RESOLVED','REJECTED')),
    resolution_type TEXT
        CHECK (resolution_type IN ('AMICABLE','MEDIATED','ARBITRATED','WITHDRAWN')),
    resolution_outcome TEXT
        CHECK (resolution_outcome IN ('IN_FAVOR_OF_RAISED','IN_FAVOR_OF_AGAINST','SPLIT','DISMISSED')),
    severity TEXT
        CHECK (severity IN ('LOW','MEDIUM','HIGH')),
    description TEXT NOT NULL,
    evidence_urls TEXT[] NOT NULL DEFAULT '{}',
    admin_notes TEXT,
    resolution_notes TEXT,
    resolved_by_user_id UUID REFERENCES users(id),
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_disputes_unique_open
    ON disputes(deal_id, raised_by_party_id)
    WHERE dispute_status IN ('OPEN','UNDER_REVIEW','MEDIATION','ESCALATED');

CREATE INDEX IF NOT EXISTS idx_disputes_deal ON disputes(deal_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_disputes_raised_by ON disputes(raised_by_party_id, dispute_status);
CREATE INDEX IF NOT EXISTS idx_disputes_against ON disputes(against_party_id, dispute_status);
CREATE INDEX IF NOT EXISTS idx_disputes_status ON disputes(dispute_status, created_at DESC);

CREATE TABLE IF NOT EXISTS dispute_responses (
    id UUID PRIMARY KEY,
    dispute_id UUID NOT NULL REFERENCES disputes(id) ON DELETE CASCADE,
    party_id UUID NOT NULL REFERENCES parties(id),
    user_id UUID NOT NULL REFERENCES users(id),
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_dispute_responses_dispute
    ON dispute_responses(dispute_id, created_at ASC);

UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['disputes:read', 'disputes:write'])
)
WHERE name = 'user';

UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['admin:disputes'])
)
WHERE name = 'admin';
```

### 5.2 Schema notes

- `against_party_id` is optional so a party can raise a deal-wide dispute (e.g., force majeure).
- The partial unique index prevents duplicate open disputes from the same party on the same deal.
- `ON DELETE CASCADE` on `deal_id` ensures disputes are removed if a deal is deleted (deals are soft-deleted in practice, so this is a safety net).
- `TEXT[]` for `evidence_urls` matches the pattern used by `party_verifications`.

### 5.3 `dispute_responses` table

The MVP implements a dedicated `dispute_responses` table so that the back-and-forth between parties is auditable, queryable, and cleanly separated from chat messages.

```sql
CREATE TABLE IF NOT EXISTS dispute_responses (
    id UUID PRIMARY KEY,
    dispute_id UUID NOT NULL REFERENCES disputes(id) ON DELETE CASCADE,
    party_id UUID NOT NULL REFERENCES parties(id),
    user_id UUID NOT NULL REFERENCES users(id),
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Index:

```sql
CREATE INDEX IF NOT EXISTS idx_dispute_responses_dispute
    ON dispute_responses(dispute_id, created_at ASC);
```

---

## 6. Crate-by-Crate Implementation Map

### 6.1 `crates/domain`

New files:

```text
crates/domain/src/entities/dispute.rs
crates/domain/src/repositories/dispute_repository.rs
```

Updates:
- `crates/domain/src/entities/mod.rs` — add `pub mod dispute;` and `pub use dispute::*;`.
- `crates/domain/src/repositories/mod.rs` — add `pub mod dispute_repository;` and `pub use dispute_repository::*;`.
- `crates/domain/src/errors.rs` — add dispute-specific error variants.

`DisputeRepository` trait:

```rust
#[async_trait]
pub trait DisputeRepository: Send + Sync {
    async fn create(&self, dispute: &Dispute) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Dispute>, DomainError>;
    async fn list_by_deal(
        &self,
        deal_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<DisputeListResult, DomainError>;
    async fn list_admin(
        &self,
        filters: &DisputeFilters,
    ) -> Result<DisputeListResult, DomainError>;
    async fn submit_evidence(
        &self,
        id: Uuid,
        evidence_urls: Vec<String>,
        notes: Option<String>,
    ) -> Result<(), DomainError>;
    async fn add_response(
        &self,
        response: &DisputeResponse,
    ) -> Result<(), DomainError>;
    async fn list_responses(
        &self,
        dispute_id: Uuid,
    ) -> Result<Vec<DisputeResponse>, DomainError>;
    async fn escalate(
        &self,
        id: Uuid,
        escalated_by_user_id: Uuid,
        notes: Option<String>,
    ) -> Result<(), DomainError>;
    async fn resolve(
        &self,
        id: Uuid,
        resolved_by_user_id: Uuid,
        resolution: DisputeResolution,
    ) -> Result<(), DomainError>;
    async fn reject(
        &self,
        id: Uuid,
        resolved_by_user_id: Uuid,
        reason: String,
    ) -> Result<(), DomainError>;
    async fn count_open_by_party(&self, party_id: Uuid) -> Result<i64, DomainError>;
    async fn count_open_against_party(&self, party_id: Uuid) -> Result<i64, DomainError>;
}
```

New `DomainError` variants:

```rust
#[error("dispute not found")]
DisputeNotFound,

#[error("an open dispute already exists for this deal and party")]
DisputeAlreadyExists,

#[error("invalid dispute type: {message}")]
InvalidDisputeType { message: String },

#[error("invalid dispute status: {message}")]
InvalidDisputeStatus { message: String },

#[error("invalid dispute resolution: {message}")]
InvalidDisputeResolution { message: String },

#[error("dispute access denied")]
DisputeAccessDenied,
```

### 6.2 `crates/application`

New module: `crates/application/src/disputes/`

```text
crates/application/src/disputes/
  ├── mod.rs
  ├── dto.rs
  ├── raise_dispute.rs
  ├── list_deal_disputes.rs
  ├── get_dispute.rs
  ├── submit_evidence.rs
  ├── respond_to_dispute.rs
  ├── escalate_dispute.rs
  ├── resolve_dispute.rs
  ├── list_admin_disputes.rs
  └── tests.rs
```

Update `crates/application/src/lib.rs`:

```rust
pub mod disputes;
```

#### Use cases

**`RaiseDispute`**

- Validate the deal exists and is in `EXECUTING` or `ON_HOLD`.
- Validate the caller is a member of `actor_party_id` (unless admin).
- Validate `actor_party_id` participates in the deal.
- Optionally validate `against_party_id` is a different participant.
- Check no open dispute already exists for `(deal_id, raised_by_party_id)`.
- Create `Dispute` with status `OPEN`.
- Transition deal to `DISPUTED` via `Deal::transition`.
- Persist the dispute and the updated deal.
- Record `deal_history` event `DISPUTE_RAISED`.
- Increment `trust_scores.deals_disputed_count` for both raising and against parties (if `against_party_id` is set).
- Request trust-score recalculation for both parties via `TrustScoreRecalculationPort`.

**`ListDealDisputes`**

- Load deal aggregate.
- Verify caller is a participant or admin.
- Return paginated disputes for the deal.

**`GetDispute`**

- Load dispute by id.
- Load responses via `DisputeRepository::list_responses`.
- Load deal aggregate.
- Verify caller is a participant or admin.
- Return `DisputeResult` with `responses` populated.

**`SubmitEvidence`**

- Load dispute.
- Verify caller is the raising party or admin.
- Append evidence URLs and optional notes.
- Move status to `UNDER_REVIEW` if currently `OPEN`.

**`RespondToDispute`**

- Load dispute.
- Verify caller is a deal participant or admin.
- Create `DisputeResponse` via `DisputeResponse::new(...)`.
- Persist via `DisputeRepository::add_response`.
- Optionally move dispute status to `UNDER_REVIEW` if currently `OPEN`.

**`EscalateDispute`**

- Admin only.
- Load dispute.
- Move status to `ESCALATED` or `MEDIATION`.
- Record admin notes.

**`ResolveDispute`**

- Admin only.
- Load dispute.
- Apply `Dispute::resolve(...)`.
- Transition the deal to the resolver-chosen next status (`EXECUTING`, `COMPLETED`, or `CANCELLED`).
- Record `deal_history` event `DISPUTE_RESOLVED`.
- Request trust-score recalculation for raising and against parties.

**`RejectDispute`**

- Admin only.
- Load dispute.
- Apply `Dispute::reject(...)`.
- Keep the deal in `DISPUTED` unless the admin explicitly transitions it; the resolver may choose `EXECUTING` if the claim was frivolous.

**`ListAdminDisputes`**

- Admin only.
- Support filters: `status`, `deal_id`, `raised_by_party_id`, `against_party_id`.
- Return paginated results ordered by `created_at DESC`.

#### DTOs (`crates/application/src/disputes/dto.rs`)

Commands carry `actor_user_id`, `actor_party_id`, and `is_admin`, following the existing reviews/verifications pattern.

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct RaiseDisputeCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub deal_id: Uuid,
    pub against_party_id: Option<Uuid>,
    pub dispute_type: String,
    pub description: String,
    pub evidence_urls: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitEvidenceCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub dispute_id: Uuid,
    pub evidence_urls: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RespondToDisputeCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub dispute_id: Uuid,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EscalateDisputeCommand {
    pub actor_user_id: Uuid,
    pub dispute_id: Uuid,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResolveDisputeCommand {
    pub actor_user_id: Uuid,
    pub dispute_id: Uuid,
    pub resolution_type: String,
    pub resolution_outcome: String,
    pub severity: String,
    pub resolution_notes: Option<String>,
    pub next_deal_status: String, // EXECUTING | COMPLETED | CANCELLED
}

#[derive(Debug, Clone, Deserialize)]
pub struct RejectDisputeCommand {
    pub actor_user_id: Uuid,
    pub dispute_id: Uuid,
    pub reason: String,
    pub next_deal_status: Option<String>, // optional transition
}
```

Result structs:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct DisputeResponseResult {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub party_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct DisputeResult {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub raised_by_party_id: Uuid,
    pub raised_by_user_id: Uuid,
    pub against_party_id: Option<Uuid>,
    pub dispute_type: String,
    pub dispute_status: String,
    pub resolution_type: Option<String>,
    pub resolution_outcome: Option<String>,
    pub severity: Option<String>,
    pub description: String,
    pub evidence_urls: Vec<String>,
    pub admin_notes: Option<String>,
    pub resolution_notes: Option<String>,
    pub resolved_by_user_id: Option<Uuid>,
    pub resolved_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub responses: Vec<DisputeResponseResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DisputeListResult {
    pub disputes: Vec<DisputeResult>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
```

#### Error additions

In `crates/application/src/errors.rs`:

```rust
#[error("dispute not found")]
DisputeNotFound,

#[error("a dispute already exists for this deal and party")]
DisputeAlreadyExists,

#[error("dispute access denied")]
DisputeAccessDenied,
```

Map `DomainError` variants in `impl From<DomainError> for ApplicationError`:
- `DisputeNotFound` → `DisputeNotFound`
- `DisputeAlreadyExists` → `DisputeAlreadyExists`
- `DisputeAccessDenied` / `InsufficientPermissions` → `DisputeAccessDenied`
- `InvalidDisputeType` / `InvalidDisputeStatus` / `InvalidDisputeResolution` → `Validation(...)`

### 6.3 `crates/infrastructure`

New file: `crates/infrastructure/src/repositories/postgres_dispute_repository.rs`

Expose in `crates/infrastructure/src/repositories/mod.rs`.

Implementation notes:
- Wrap a `PgPool` like the other Postgres repositories.
- Use `sqlx::query!` / `sqlx::query_as!` macros.
- After the migration is applied, run `cargo sqlx prepare --workspace` to regenerate `.sqlx/` offline metadata.
- Map the unique constraint `idx_disputes_unique_open` to `DomainError::DisputeAlreadyExists` in the repository's `map_err` helper.
- For `resolve`/`reject`, use an atomic `UPDATE … WHERE id = $1 AND dispute_status NOT IN ('RESOLVED','REJECTED')` and check `rows_affected()` to detect concurrent modifications.
- When resolving, update `trust_scores.deals_disputed_count` for raising and against parties and invoke `TrustScoreRecalculationPort::request_recalculation`.
- `add_response` inserts into `dispute_responses`.
- `list_responses` queries `dispute_responses` ordered by `created_at ASC`.

### 6.4 `crates/api`

New files:

```text
crates/api/src/routes/disputes.rs
crates/api/src/handlers/disputes/
  ├── mod.rs
  ├── dto.rs
  ├── create_dispute.rs
  ├── list_deal_disputes.rs
  ├── get_dispute.rs
  ├── submit_evidence.rs
  ├── respond_to_dispute.rs
  ├── admin_resolve_dispute.rs
  ├── admin_reject_dispute.rs
  ├── admin_escalate_dispute.rs
  └── admin_list_disputes.rs
```

Wire in `crates/api/src/routes/mod.rs`:

```rust
pub mod disputes;
// ...
.configure(disputes::configure)
```

Update `crates/api/src/lib.rs` `AppState` to add dispute use cases.

Update `crates/api/src/main.rs` to construct the `PostgresDisputeRepository` and the dispute use cases, and inject them into `AppState`.

Routes (`crates/api/src/routes/disputes.rs`):

```rust
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/deals/{deal_id}/disputes")
            .route(web::post().to(create_dispute::create_dispute))
            .route(web::get().to(list_deal_disputes::list_deal_disputes)),
    )
    .service(
        web::resource("/disputes/{dispute_id}")
            .route(web::get().to(get_dispute::get_dispute)),
    )
    .service(
        web::resource("/disputes/{dispute_id}/evidence")
            .route(web::post().to(submit_evidence::submit_evidence)),
    )
    .service(
        web::resource("/disputes/{dispute_id}/responses")
            .route(web::post().to(respond_to_dispute::respond_to_dispute)),
    )
    .service(
        web::resource("/admin/disputes")
            .route(web::get().to(admin_list_disputes::admin_list_disputes)),
    )
    .service(
        web::resource("/admin/disputes/{dispute_id}/escalate")
            .route(web::post().to(admin_escalate_dispute::admin_escalate_dispute)),
    )
    .service(
        web::resource("/admin/disputes/{dispute_id}/resolve")
            .route(web::post().to(admin_resolve_dispute::admin_resolve_dispute)),
    )
    .service(
        web::resource("/admin/disputes/{dispute_id}/reject")
            .route(web::post().to(admin_reject_dispute::admin_reject_dispute)),
    );
}
```

---

## 7. API Contract

### 7.1 Raise a dispute

```http
POST /api/v1/deals/{deal_id}/disputes
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json
```

Request:

```json
{
  "against_party_id": "550e8400-e29b-41d4-a716-446655440000",
  "dispute_type": "QUALITY_ISSUE",
  "description": "Delivered produce did not meet the agreed quality grade.",
  "evidence_urls": ["https://storage.example.com/evidence1.jpg"]
}
```

Response `201 Created`:

```json
{
  "id": "dispute-uuid",
  "deal_id": "deal-uuid",
  "raised_by_party_id": "party-uuid",
  "raised_by_user_id": "user-uuid",
  "against_party_id": "550e8400-e29b-41d4-a716-446655440000",
  "dispute_type": "QUALITY_ISSUE",
  "dispute_status": "OPEN",
  "description": "Delivered produce did not meet the agreed quality grade.",
  "evidence_urls": ["https://storage.example.com/evidence1.jpg"],
  "created_at": "2026-06-15T10:00:00Z",
  "updated_at": "2026-06-15T10:00:00Z"
}
```

### 7.2 List disputes for a deal

```http
GET /api/v1/deals/{deal_id}/disputes?limit=20&offset=0
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Response `200 OK`:

```json
{
  "disputes": [ /* ... */ ],
  "total": 1,
  "limit": 20,
  "offset": 0
}
```

### 7.3 Get a dispute

```http
GET /api/v1/disputes/{dispute_id}
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

### 7.4 Submit evidence

```http
POST /api/v1/disputes/{dispute_id}/evidence
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json
```

Request:

```json
{
  "evidence_urls": ["https://storage.example.com/evidence2.jpg"],
  "notes": "Additional photos from the delivery site."
}
```

### 7.5 Respond to a dispute

```http
POST /api/v1/disputes/{dispute_id}/responses
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json
```

Request:

```json
{
  "message": "The quality was inspected and accepted at pickup."
}
```

### 7.6 Admin: list disputes

```http
GET /api/v1/admin/disputes?status=OPEN&deal_id=...&limit=20&offset=0
Authorization: Bearer <jwt>
```

### 7.7 Admin: escalate

```http
POST /api/v1/admin/disputes/{dispute_id}/escalate
Authorization: Bearer <jwt>
Content-Type: application/json
```

Request:

```json
{
  "notes": "Parties unable to reach agreement; assign senior mediator."
}
```

### 7.8 Admin: resolve

```http
POST /api/v1/admin/disputes/{dispute_id}/resolve
Authorization: Bearer <jwt>
Content-Type: application/json
```

Request:

```json
{
  "resolution_type": "MEDIATED",
  "resolution_outcome": "SPLIT",
  "severity": "MEDIUM",
  "resolution_notes": "Supplier to replace 30% of the shipment; consumer releases 70% of escrow.",
  "next_deal_status": "EXECUTING"
}
```

### 7.9 Admin: reject

```http
POST /api/v1/admin/disputes/{dispute_id}/reject
Authorization: Bearer <jwt>
Content-Type: application/json
```

Request:

```json
{
  "reason": "No evidence provided; claim appears frivolous.",
  "next_deal_status": "EXECUTING"
}
```

### 7.10 Error mapping

| Situation | HTTP status | Error code |
|-----------|-------------|------------|
| Deal not found | 404 | `deal_not_found` |
| Dispute not found | 404 | `dispute_not_found` |
| Caller not a participant | 403 | `forbidden` |
| Missing scope | 403 | `forbidden` |
| Deal not in disputable state | 422 | `invalid_state_transition` |
| Duplicate open dispute | 409 | `dispute_already_exists` |
| Invalid enum value | 422 | `validation` |
| Concurrent modification (already resolved) | 409 | `validation` |

---

## 8. Trust Score Integration

The trust-score system (`trust-score.md`) already defines a `dispute_history` component with weight 10% and a `deals_disputed_count` column.

### 8.1 On dispute raised

1. Increment `trust_scores.deals_disputed_count` for the raising party.
2. If `against_party_id` is set, increment `trust_scores.deals_disputed_count` for the against party.
3. Call `TrustScoreRecalculationPort::request_recalculation(party_id)` for both.

### 8.2 On dispute resolved

1. Record `resolution_type` and `severity` on the dispute.
2. Call `TrustScoreRecalculationPort::request_recalculation(party_id)` for both parties.

### 8.3 Penalty table

The trust-score calculator uses the following penalty factors (from `trust-score.md` §5.6):

| Resolution type | Penalty factor |
|-----------------|----------------|
| Resolved amicably | 5 |
| Mediated resolution | 10 |
| Lost arbitration | 20 |

Pattern penalties:

| Pattern | Penalty |
|---------|---------|
| 3+ disputes in 6 months | +10 |
| 5+ disputes in 6 months | +25 |

The recalculation job computes `DISP_Score = max(0, 100 - Dispute_Penalty)`.

---

## 9. Deal Lifecycle Integration

### 9.1 Raising a dispute

1. Load deal aggregate.
2. Validate `deal_status == EXECUTING || deal_status == ON_HOLD`.
3. Call `deal.transition(Disputed)`.
4. Persist the deal.
5. Record `deal_history` with event type `DISPUTE_RAISED`, `actor_party_id`, and dispute id.

### 9.2 Resolving a dispute

1. Load dispute and deal.
2. Apply `Dispute::resolve(...)`.
3. Parse `next_deal_status` from the resolver's command.
4. Call `deal.transition(next_status)` where `next_status ∈ {Executing, Completed, Cancelled}`.
5. Persist the deal.
6. Record `deal_history` with event type `DISPUTE_RESOLVED` and resolution metadata.

### 9.3 Timeout handling

No change is required to `ProcessDealTimeouts`. The existing mapping already transitions `DISPUTED → ON_HOLD` after `disputed_seconds` (default 14 days). When a dispute is resolved before the timeout, the deal leaves `DISPUTED` and the timeout no longer applies.

---

## 10. Testing Strategy

Target: **>86% line coverage** for all new dispute code.

### 10.1 Domain unit tests

File: `crates/domain/src/entities/dispute.rs` (`#[cfg(test)]` module).

Cases:
- `Dispute::new` creates a dispute with status `OPEN`.
- `submit_for_review` moves `OPEN → UNDER_REVIEW`.
- `escalate` moves `UNDER_REVIEW → ESCALATED`.
- `resolve` moves to `RESOLVED` and sets all resolution fields.
- `reject` moves to `REJECTED`.
- Invalid transitions return `InvalidDisputeStatus`.
- Enum round-trips (`as_str` / `TryFrom<&str>`) work for all variants.

### 10.2 Application unit tests

File: `crates/application/src/disputes/tests.rs`.

Add `FakeDisputeRepo` to `crates/application/src/test_helpers.rs` implementing `DisputeRepository` with an in-memory `HashMap<Uuid, Dispute>`.

Cases:
- Raise dispute succeeds and transitions deal to `DISPUTED`.
- Raise dispute fails when deal is `COMPLETED`.
- Raise dispute fails for non-participant.
- Raise dispute fails when an open dispute already exists.
- Admin can raise on behalf of any party.
- List/get disputes enforce participant/admin access.
- Submit evidence allowed only for raising party or admin.
- Respond allowed for any participant.
- Escalate/resolve/reject allowed only for admin.
- Resolve transitions deal to chosen next status.
- Trust-score recalculation port is invoked for both parties.

### 10.3 Postgres integration tests

File: `crates/infrastructure/src/repositories/tests/dispute_repository.rs`.

Cases:
- Create and find a dispute.
- List by deal with pagination.
- List admin with filters.
- Submit evidence updates `evidence_urls` and `admin_notes`.
- Escalate updates status.
- Resolve updates status and resolution fields.
- Unique open-dispute constraint prevents duplicates.
- `cargo sqlx prepare` metadata is up to date.

### 10.4 API tests

File: `crates/api/tests/disputes.rs`.

Cases:
- `disputes:write` scope required to raise; `disputes:read` to list.
- `admin:disputes` or `admin:*` grants admin endpoints.
- Non-participant receives 403.
- End-to-end: raise → list → get → submit evidence → respond → escalate → resolve.

### 10.5 Coverage enforcement

After implementation:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo sqlx prepare --workspace
# Optional coverage check
cargo tarpaulin --workspace --ignore-tests
```

Aim for >86% line coverage. If coverage is below target, add tests for error branches and access-control checks.

---

## 11. Security & Privacy Considerations

1. **Evidence URLs are not verified.** The API accepts arbitrary HTTPS URLs. Future work: validate URL format, scan for malicious content, store files in platform-controlled storage.
2. **Dispute descriptions may contain sensitive information.** Access is restricted to deal participants and admins.
3. **Admin actions are audited.** Every escalation, resolution, and rejection records `resolved_by_user_id` and `resolved_at`.
4. **No PII leakage in list endpoints.** Results include only dispute metadata, not internal admin notes unless the caller is an admin.

---

## 12. Open Questions & Future Work

1. **Good-faith deposit.** `3partydeal.pdf` mentions a 2% good-faith deposit to raise a dispute. The current ledger does not support this automatically; document as a future enhancement tied to wallet/escrow logic.
2. **Escrow freezing.** While a deal is `DISPUTED`, `ESCROW_RELEASE` transaction approvals should be blocked. Add this check in `ApproveTransaction` once transactions are fully implemented.
3. **Two-party mediation workflow.** Split `MEDIATION` and `ESCALATED` into distinct states with assigned mediators and scheduled sessions.
4. **Notifications.** Send email/in-app notifications to all parties when a dispute is raised, escalated, resolved, or receives a new response.
5. **Real-time updates.** Publish dispute events through the existing `RealtimePublisher` so dashboards update live.

---

## 13. Summary Checklist

- [ ] Add `Dispute` and `DisputeResponse` entities and enums in `crates/domain`.
- [ ] Add `DisputeRepository` port in `crates/domain` (including response methods).
- [ ] Add dispute `DomainError` variants.
- [ ] Create `crates/application/src/disputes/` module with DTOs and use cases.
- [ ] Add dispute `ApplicationError` variants and mappings.
- [ ] Create `PostgresDisputeRepository` in `crates/infrastructure`.
- [ ] Create migration `migrations/20260615160000_create_disputes.sql` with `disputes` and `dispute_responses` tables and seed scopes.
- [ ] Create API routes/handlers in `crates/api`.
- [ ] Wire dispute use cases into `AppState` and `main.rs`.
- [ ] Update `trust_scores.deals_disputed_count` and trigger recalculation.
- [ ] Add `FakeDisputeRepo` to application test helpers.
- [ ] Add domain, application, Postgres, and API tests.
- [ ] Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo sqlx prepare --workspace`.
- [ ] Verify test coverage is above 86%.
