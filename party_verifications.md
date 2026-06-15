# Party Verifications — Implementation Specification

> **Scope:** Define how Hayaland verifies the real-world identity and legitimacy of Parties, and how those verifications feed trust scoring, search ranking, and platform eligibility.  
> **Audience:** Backend engineers implementing the feature.  
> **Depends on:** Existing `Party`, `User`, `UserPartyMembership`, JWT/`X-Party-ID` auth, `parties.verification_status`, and `trust_scores.verification_level`.  
> **Leads to:** Trust-score recalculation (`trust-score.md` §5.4) and platform eligibility checks.

---

## 1. Overview

A **Party Verification** is a platform-approved claim that a Party has completed a specific real-world check. Verifications are the primary mechanism for turning an anonymous Party into a trusted platform participant.

The implementation mirrors the existing **Review feature** (`crates/application/src/reviews/`, `crates/api/src/handlers/reviews/`, etc.) and reuses the same hexagonal crate boundaries, auth patterns, error handling, and repository abstractions.

### 1.1 MVP goals

1. A Party can have **multiple verifications**, one per `verification_type`.
2. Verification requests are submitted by **members of the Party**; approval/rejection is performed by **platform admins**.
3. Each verification type maps to a **numeric weight** that contributes to the Party's `verification_level` (0–5) stored in `trust_scores.verification_level`.
4. The high-level state is reflected on `parties.verification_status` (`UNVERIFIED`, `PENDING`, `VERIFIED`, `REJECTED`) using the enum already defined in `crates/domain/src/entities/party.rs`.
5. Approving, rejecting, or revoking a verification triggers a **trust-score recalculation** for the Party via the existing `TrustScoreRecalculationPort` pattern.
6. Verification evidence is stored in object storage; `evidence_urls` are only visible to the submitting Party and admins.
7. Write/admin endpoints require the `admin:verifications` scope (or `admin:*`); Party members use `verifications:write` and `verifications:read`.

### 1.2 Out of scope

- Automated KYC provider integrations (Jumio, Onfido, Plaid, etc.). Keep a `provider_reference`/`provider_payload` hook for future adapters.
- Video verification interviews.
- Biometric verification.
- Third-party notarization.
- Public verification badges beyond `parties.verification_status`.
- Automatic expiry reminders and grace-period workflows.
- Charging fees for verification.

---

## 2. Relationship to Existing Code

### 2.1 Existing `Party` aggregate

`crates/domain/src/entities/party.rs` already defines:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    Unverified,
    Pending,
    Verified,
    Rejected,
}

pub struct Party {
    pub id: Uuid,
    pub party_type: PartyType,
    pub display_name: DisplayName,
    pub email: Email,
    pub phone: Option<Phone>,
    pub tax_id: Option<String>,
    pub verification_status: VerificationStatus,  // already exists
    pub primary_domain_id: Option<Uuid>,
    pub location: Option<GeoPoint>,
    pub service_radius_km: Option<f64>,
    pub trust_score: f64,
    pub total_deals_completed: i32,
    pub total_deals_initiated: i32,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

This enum is reused as the high-level Party status. A separate `PartyVerificationStatus` enum is **not** needed.

### 2.2 Existing `parties` table

`migrations/20260613000000_create_parties_table.sql` already has:

```sql
verification_status TEXT NOT NULL DEFAULT 'UNVERIFIED'
    CHECK (verification_status IN ('UNVERIFIED','PENDING','VERIFIED','REJECTED')),
```

### 2.3 Existing `trust_scores` table

`migrations/20260613014000_create_agreements_signatures_reviews_trust.sql` has:

```sql
CREATE TABLE IF NOT EXISTS trust_scores (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL UNIQUE REFERENCES parties(id) ON DELETE CASCADE,
    ...
    verification_level INTEGER NOT NULL DEFAULT 0,
    ...
);
```

The Party Verifications feature is the source of truth for `verification_level`.

### 2.4 Existing recalculation port

`crates/application/src/reviews/submit_review.rs` defines the outbound port already used by Reviews:

```rust
#[async_trait]
pub trait TrustScoreRecalculationPort: Send + Sync {
    async fn request_recalculation(&self, party_id: Uuid) -> Result<(), ApplicationError>;
}

pub struct NoOpTrustScoreRecalculation;
#[async_trait]
impl TrustScoreRecalculationPort for NoOpTrustScoreRecalculation {
    async fn request_recalculation(&self, _party_id: Uuid) -> Result<(), ApplicationError> { Ok(()) }
}
```

Party Verifications must reuse this exact trait. The `NoOpTrustScoreRecalculation` is wired in `crates/api/src/main.rs` until the trust-score use case exists.

---

## 3. Verification Types & Level Mapping

Each verification record has a `verification_type`. The platform starts with the following types.

| Type | Code | Points | Admin Review | Evidence |
|---|---|---|---|---|
| Email verification | `EMAIL` | 10 | No | Reuse existing `email_verifications` table; create a `party_verifications` record when email is verified. |
| Phone verification | `PHONE` | 15 | No | Implement OTP flow; create record on success. |
| Government ID | `GOVERNMENT_ID` | 30 | Yes | Passport, national ID, driver's license. |
| Business registration | `BUSINESS_REGISTRATION` | 25 | Yes | Business licence, tax certificate, registry extract. |
| Bank account | `BANK_ACCOUNT` | 10 | Yes | Bank statement or micro-deposit confirmation. |
| Professional certification | `PROFESSIONAL_CERTIFICATION` | 10 | Yes | Certificate document; may be tied to a `party_role`. |
| Video interview | `VIDEO_INTERVIEW` | +10 bonus | Yes | Completed interview record; bonus that can push raw score above 100 but final is capped at 100. |

### 3.1 Verification level mapping

Effective points = sum of approved, non-expired verification points.

| Level | Minimum Effective Points | Meaning |
|---|---|---|
| 0 | 0 | None |
| 1 | 10 | Email verified |
| 2 | 25 | Email + Phone |
| 3 | 55 | + Government ID |
| 4 | 80 | + Business registration |
| 5 | 100 | + Bank account + certification |

The video interview bonus adds to the raw score but the effective level is capped at 5 when the non-bonus base reaches 100.

---

## 4. Data Model

### 4.1 `party_verifications` table

New migration (follow timestamp convention, e.g. `20260615130000_create_party_verifications.sql`):

```sql
CREATE TYPE verification_status_enum AS ENUM (
    'PENDING', 'APPROVED', 'REJECTED', 'EXPIRED', 'REVOKED'
);

CREATE TABLE IF NOT EXISTS party_verifications (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
    requested_by_user_id UUID NOT NULL REFERENCES users(id),
    reviewed_by_user_id UUID REFERENCES users(id),
    verification_type TEXT NOT NULL
        CHECK (verification_type IN (
            'EMAIL', 'PHONE', 'GOVERNMENT_ID', 'BUSINESS_REGISTRATION',
            'BANK_ACCOUNT', 'PROFESSIONAL_CERTIFICATION', 'VIDEO_INTERVIEW'
        )),
    status verification_status_enum NOT NULL DEFAULT 'PENDING',
    points INTEGER NOT NULL,
    evidence_urls TEXT[] NOT NULL DEFAULT '{}',
    provider_reference TEXT,
    provider_payload JSONB,
    rejection_reason TEXT,
    review_notes TEXT,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reviewed_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (party_id, verification_type, status)  -- one approved/pending per type
);

CREATE INDEX IF NOT EXISTS idx_party_verifications_party
    ON party_verifications(party_id, status, verification_type);
CREATE INDEX IF NOT EXISTS idx_party_verifications_status
    ON party_verifications(status, requested_at);
CREATE INDEX IF NOT EXISTS idx_party_verifications_type
    ON party_verifications(verification_type, status);
```

> **Note:** The unique constraint above is illustrative. If the design allows re-submitting after rejection/expiry, use a partial unique index instead:
> ```sql
> CREATE UNIQUE INDEX IF NOT EXISTS idx_party_verifications_unique_active
>     ON party_verifications(party_id, verification_type)
>     WHERE status IN ('PENDING', 'APPROVED');
> ```

### 4.2 Evidence handling

- Evidence files are uploaded to object storage via a separate upload endpoint.
- `evidence_urls` stores object keys or presigned URLs.
- Access rules:
  - Party members can view their own Party's evidence URLs.
  - Admins (`admin:verifications` or `admin:*`) can view any evidence URLs.
  - Public users see only type, status, and level; evidence URLs are omitted.

### 4.3 Party status synchronization

When a verification is approved/rejected/revoked:

1. Recompute effective points from `party_verifications` where `status = 'APPROVED'` and `(expires_at IS NULL OR expires_at > now())`.
2. Map effective points to `verification_level` and update `trust_scores.verification_level`.
3. Update `parties.verification_status` using the existing enum:
   - `UNVERIFIED` — no approved verifications and no pending ones.
   - `PENDING` — no approved verifications but at least one pending.
   - `VERIFIED` — at least one approved verification.
   - `REJECTED` — only rejected/revoked/expired exist (edge case; prefer `UNVERIFIED`).
4. Trigger trust-score recalculation via `TrustScoreRecalculationPort::request_recalculation(party_id)`.

---

## 5. Domain Layer

### 5.1 New entity

**File:** `crates/domain/src/entities/party_verification.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PartyVerificationType {
    Email,
    Phone,
    GovernmentId,
    BusinessRegistration,
    BankAccount,
    ProfessionalCertification,
    VideoInterview,
}

impl PartyVerificationType {
    pub fn as_str(&self) -> &'static str { ... }
    pub fn points(&self) -> i32 { ... }
}

impl TryFrom<&str> for PartyVerificationType { ... }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PartyVerificationStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Revoked,
}

impl PartyVerificationStatus {
    pub fn as_str(&self) -> &'static str { ... }
}

impl TryFrom<&str> for PartyVerificationStatus { ... }

pub struct PartyVerification {
    pub id: Uuid,
    pub party_id: Uuid,
    pub requested_by_user_id: Uuid,
    pub reviewed_by_user_id: Option<Uuid>,
    pub verification_type: PartyVerificationType,
    pub status: PartyVerificationStatus,
    pub points: i32,
    pub evidence_urls: Vec<String>,
    pub provider_reference: Option<String>,
    pub provider_payload: Option<Value>,
    pub rejection_reason: Option<String>,
    pub review_notes: Option<String>,
    pub requested_at: OffsetDateTime,
    pub reviewed_at: Option<OffsetDateTime>,
    pub expires_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

Register in `crates/domain/src/entities/mod.rs`:

```rust
pub mod party_verification;
pub use party_verification::*;
```

### 5.2 Domain errors

Add to `crates/domain/src/errors.rs`:

```rust
#[error("invalid verification type: {message}")]
InvalidVerificationType { message: String },

#[error("verification not found")]
VerificationNotFound,

#[error("a verification already exists for this party and type")]
DuplicateVerification,

#[error("invalid verification state transition from {from} to {to}")]
InvalidVerificationStateTransition { from: String, to: String },

#[error("rejection reason is required")]
MissingRejectionReason,

#[error("verification evidence is required")]
MissingVerificationEvidence,
```

### 5.3 Repository port

**File:** `crates/domain/src/repositories/party_verification_repository.rs`

```rust
#[async_trait]
pub trait PartyVerificationRepository: Send + Sync {
    async fn create(&self, verification: &PartyVerification) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PartyVerification>, DomainError>;
    async fn find_active_by_party_and_type(
        &self,
        party_id: Uuid,
        verification_type: PartyVerificationType,
    ) -> Result<Option<PartyVerification>, DomainError>;
    async fn list_by_party(
        &self,
        party_id: Uuid,
    ) -> Result<Vec<PartyVerification>, DomainError>;
    async fn list_pending(
        &self,
        filters: &VerificationListFilters,
    ) -> Result<VerificationListResult, DomainError>;
    async fn count_pending(
        &self,
        filters: &VerificationListFilters,
    ) -> Result<i64, DomainError>;
    async fn approve(
        &self,
        id: Uuid,
        reviewed_by_user_id: Uuid,
        review_notes: Option<String>,
    ) -> Result<(), DomainError>;
    async fn reject(
        &self,
        id: Uuid,
        reviewed_by_user_id: Uuid,
        rejection_reason: String,
        review_notes: Option<String>,
    ) -> Result<(), DomainError>;
    async fn revoke(
        &self,
        id: Uuid,
        reviewed_by_user_id: Uuid,
        reason: String,
        review_notes: Option<String>,
    ) -> Result<(), DomainError>;
    async fn sum_approved_points(&self, party_id: Uuid) -> Result<i64, DomainError>;
}
```

Register in `crates/domain/src/repositories/mod.rs`:

```rust
pub mod party_verification_repository;
pub use party_verification_repository::*;
```

---

## 6. Application Layer

### 6.1 Module layout

`crates/application/src/verifications/`:

```
mod.rs
dto.rs
submit_verification.rs
list_party_verifications.rs
get_verification_status.rs
approve_verification.rs
reject_verification.rs
revoke_verification.rs
list_admin_verifications.rs
```

Register in `crates/application/src/lib.rs`:

```rust
pub mod verifications;
```

### 6.2 Outbound port

Reuse `TrustScoreRecalculationPort` from `crates/application/src/reviews/submit_review.rs`. If the trait is moved to a shared location later, update imports. For now, duplicate or re-export from a shared `ports` module (preferred: `crates/application/src/ports.rs`).

### 6.3 Use cases

| Use case | Responsibility |
|---|---|
| `SubmitVerification` | Member submits a verification. Validates type, duplicate active checks, evidence, creates `PENDING` record. |
| `ListPartyVerifications` | List verifications for a Party; filters evidence based on caller. |
| `GetVerificationStatus` | Return `verification_level`, `verification_status`, effective points, and counts. |
| `ApproveVerification` | Admin approves a pending verification; updates status; triggers recalc. |
| `RejectVerification` | Admin rejects a pending verification with a reason; triggers recalc if needed. |
| `RevokeVerification` | Admin invalidates an approved verification; triggers recalc. |
| `ListAdminVerifications` | Admin queue with filters (`status`, `verification_type`, pagination). |

All use cases accept commands carrying `actor_user_id`, `actor_party_id`, and `is_admin`, matching `SubmitReview`.

### 6.4 Application errors

Add to `crates/application/src/errors.rs`:

```rust
#[error("a verification already exists for this party and type")]
DuplicateVerification,

#[error("verification not found")]
VerificationNotFound,
```

Map domain variants in `impl From<DomainError> for ApplicationError`:

```rust
DomainError::VerificationNotFound => ApplicationError::VerificationNotFound,
DomainError::DuplicateVerification => ApplicationError::DuplicateVerification,
DomainError::InvalidVerificationType { .. }
| DomainError::InvalidVerificationStateTransition { .. }
| DomainError::MissingRejectionReason
| DomainError::MissingVerificationEvidence => ApplicationError::ValidationError(err.to_string()),
```

---

## 7. Infrastructure Layer

### 7.1 Postgres repository

**File:** `crates/infrastructure/src/repositories/postgres_party_verification_repository.rs`

```rust
pub struct PostgresPartyVerificationRepository {
    pool: PgPool,
}

impl PostgresPartyVerificationRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl PartyVerificationRepository for PostgresPartyVerificationRepository { ... }
```

Implementation notes:

- Use `sqlx::query!` for writes.
- Use a private `PartyVerificationRow` + `sqlx::query_as!` for reads, then map to the domain entity.
- Map `idx_party_verifications_unique_active` violations to `DomainError::DuplicateVerification` in `map_err`.
- `sum_approved_points` should exclude expired rows: `expires_at IS NULL OR expires_at > now()`.

Register in `crates/infrastructure/src/repositories/mod.rs`.

### 7.2 Wiring in `main.rs`

```rust
let party_verification_repo: Arc<dyn PartyVerificationRepository> =
    Arc::new(PostgresPartyVerificationRepository::new(pool.clone()));
```

Construct use cases in `crates/api/src/main.rs` and attach to `AppState` in `crates/api/src/lib.rs`, following the Review pattern.

---

## 8. API Layer

### 8.1 Routes

**File:** `crates/api/src/routes/verifications.rs`

```rust
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/parties/{party_id}/verifications")
            .route(web::post().to(create_verification::create_verification))
            .route(web::get().to(list_party_verifications::list_party_verifications)),
    )
    .service(
        web::resource("/parties/{party_id}/verifications/status")
            .route(web::get().to(get_verification_status::get_verification_status)),
    )
    .service(
        web::resource("/admin/verifications")
            .route(web::get().to(admin_list_verifications::admin_list_verifications)),
    )
    .service(
        web::resource("/admin/verifications/{verification_id}/approve")
            .route(web::post().to(admin_approve_verification::admin_approve_verification)),
    )
    .service(
        web::resource("/admin/verifications/{verification_id}/reject")
            .route(web::post().to(admin_reject_verification::admin_reject_verification)),
    )
    .service(
        web::resource("/admin/verifications/{verification_id}/revoke")
            .route(web::post().to(admin_revoke_verification::admin_revoke_verification)),
    );
}
```

Wire into `crates/api/src/routes/mod.rs`:

```rust
.configure(verifications::configure)
```

### 8.2 Handlers

**Directory:** `crates/api/src/handlers/verifications/`

```
mod.rs
create_verification.rs
list_party_verifications.rs
get_verification_status.rs
admin_list_verifications.rs
admin_approve_verification.rs
admin_reject_verification.rs
admin_revoke_verification.rs
dto.rs
```

Register in `crates/api/src/handlers/mod.rs`:

```rust
pub mod verifications;
```

Handler pattern (same as `crates/api/src/handlers/reviews/create_review.rs`):

```rust
pub async fn create_verification(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<CreateVerificationRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = req.extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(ApplicationError::Unauthorized))?;

    require_scope_or_admin(&ctx, "verifications:write", "admin:verifications")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let is_admin = ctx.has_scope("admin:verifications") || ctx.has_scope("admin:*");

    let cmd = SubmitVerificationCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        target_party_id: path.into_inner(),
        is_admin,
        verification_type: body.verification_type,
        evidence_urls: body.evidence_urls.clone(),
        notes: body.notes.clone(),
    };

    let result = state.submit_verification.execute(cmd).await?;
    Ok(HttpResponse::Created().json(VerificationResponse::from(result)))
}
```

### 8.3 DTOs

**File:** `crates/api/src/handlers/verifications/dto.rs`

- Request DTOs derive `validator::Validate` and use `#[serde(rename_all = "camelCase")]`.
- Response DTOs use `#[serde(rename_all = "camelCase")]`.
- `From<application::verifications::dto::VerificationResult>` impls map application → API response.
- Evidence URLs are only populated when the caller is a member or admin.

---

## 9. Authorization Model

### 9.1 Scopes

| Scope | Role | Purpose |
|---|---|---|
| `verifications:read` | `user` | View own Party's verifications. |
| `verifications:write` | `user` | Submit verification requests. |
| `admin:verifications` | `admin` | Approve, reject, revoke, list queue, view evidence. |
| `admin:*` | `admin` | Implicitly grants `admin:verifications`. |

### 9.2 Scope migration

Same pattern as `20260615120000_review_indexes_constraints_and_scopes.sql`:

```sql
UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['verifications:read', 'verifications:write'])
)
WHERE name = 'user';

UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['admin:verifications'])
)
WHERE name = 'admin';
```

### 9.3 Auth helpers

Use existing helpers from `crates/api/src/middleware/auth.rs`:

```rust
require_scope_or_admin(&ctx, "verifications:write", "admin:verifications")?;
require_scope_or_admin(&ctx, "verifications:read", "admin:verifications")?;
require_scope_or_admin(&ctx, "admin:verifications", "admin:verifications")?; // admin-only
```

Membership checks are performed in the application use case via `PartyRepository::is_user_member_of_party` or `find_membership`, matching `crates/application/src/parties/update_party.rs`.

---

## 10. Trust-Score Integration

### 10.1 Inputs to trust calculator

The trust-score use case reads from `party_verifications`:

```sql
SELECT verification_type, points, reviewed_at
FROM party_verifications
WHERE party_id = $1
  AND status = 'APPROVED'
  AND (expires_at IS NULL OR expires_at > now());
```

### 10.2 Scoring

Per `trust-score.md` §5.4:

```text
Effective_Points = SUM(points) for all approved, non-expired verifications
Raw_VER_Score = min(100, Effective_Points + VideoInterview_bonus)
verification_level = lookup(Effective_Points)  -- 0..5
```

### 10.3 Recalculation trigger

`ApproveVerification`, `RejectVerification`, and `RevokeVerification` call:

```rust
self.recalc.request_recalculation(party_id).await?;
```

This is the same pattern used by `SubmitReview`.

---

## 11. Verification Lifecycle

```text
Party member submits verification
            │
            ▼
    ┌───────────────┐
    │ status=PENDING │
    └───────────────┘
            │
    ┌───────┴───────┐
    ▼               ▼
 Admin approves   Admin rejects
    │               │
    ▼               ▼
┌─────────┐    ┌──────────┐
│APPROVED │    │ REJECTED │
└────┬────┘    └──────────┘
     │               │
     ▼               ▼
 trust-score    trust-score
 recalc (if     recalc (if
 approved)      it changes
                high-level status)
```

`EXPIRED` and `REVOKED` are terminal states for previously approved records and both trigger recalculation.

---

## 12. Validation Rules

| # | Rule | Domain / Application Error |
|---|---|---|
| 1 | `verification_type` must be supported. | `InvalidVerificationType` |
| 2 | A Party cannot have another `PENDING` or `APPROVED` record of the same type. | `DuplicateVerification` |
| 3 | Admin-review types require at least one `evidence_url`. | `MissingVerificationEvidence` |
| 4 | Rejection requires a `rejection_reason`. | `MissingRejectionReason` |
| 5 | Only `PENDING` can be approved or rejected. | `InvalidVerificationStateTransition` |
| 6 | Only `APPROVED` can be revoked. | `InvalidVerificationStateTransition` |
| 7 | Caller must be a member of the target Party, unless admin. | `ApplicationError::Forbidden` |

---

## 13. Edge Cases

- **Duplicate submission while pending:** reject with `DomainError::DuplicateVerification` → HTTP 409.
- **Approval after expiry:** not allowed; request a new submission.
- **Party soft-deleted:** retain records for audit; block new submissions.
- **Role-specific certifications:** optionally store `party_role_id`; revoke if role is removed.
- **Points rebalancing:** store current `points` on the record but recompute level from the latest type weights during recalculation.

---

## 14. Testing Strategy

### 14.1 Domain tests

- `PartyVerificationType::try_from` and `points()`.
- `PartyVerificationStatus::try_from` and `as_str()`.

### 14.2 Application tests

- Member submits verification.
- Non-member cannot submit.
- Duplicate active type rejected.
- Admin approve/reject/revoke.
- `verification_level` recomputed after each state change.
- `TrustScoreRecalculationPort` called on approval.

### 14.3 Infrastructure tests

- Create, find, list, approve, reject, revoke with filters.
- Expired rows excluded from `sum_approved_points`.
- Unique active index enforcement.

### 14.4 API integration tests

- End-to-end submit → admin approve → status summary.
- Evidence URLs hidden from non-members.
- Scope-based admin access control.

---

## 15. File Map

| Layer | File |
|---|---|
| Migration | `migrations/20260615130000_create_party_verifications.sql` (or next available timestamp) |
| Domain entity | `crates/domain/src/entities/party_verification.rs` |
| Domain repo port | `crates/domain/src/repositories/party_verification_repository.rs` |
| Domain errors | `crates/domain/src/errors.rs` |
| Application DTOs | `crates/application/src/verifications/dto.rs` |
| Application use cases | `crates/application/src/verifications/{submit_verification,list_party_verifications,get_verification_status,approve_verification,reject_verification,revoke_verification,list_admin_verifications}.rs` |
| Application module | `crates/application/src/verifications/mod.rs` + `crates/application/src/lib.rs` |
| Application errors | `crates/application/src/errors.rs` |
| Infrastructure repo | `crates/infrastructure/src/repositories/postgres_party_verification_repository.rs` |
| Infrastructure module | `crates/infrastructure/src/repositories/mod.rs` |
| API routes | `crates/api/src/routes/verifications.rs` |
| API handlers | `crates/api/src/handlers/verifications/*.rs` |
| API modules | `crates/api/src/handlers/mod.rs`, `crates/api/src/routes/mod.rs` |
| API state | `crates/api/src/lib.rs`, `crates/api/src/main.rs` |
| API errors | `crates/api/src/errors.rs` |

---

## 16. References

- `crates/domain/src/entities/party.rs` — existing `VerificationStatus` enum.
- `crates/application/src/reviews/` — canonical feature implementation pattern.
- `crates/api/src/handlers/reviews/` — handler and auth patterns.
- `crates/api/src/middleware/auth.rs` — scope helpers.
- `migrations/20260613000000_create_parties_table.sql` — `verification_status` column.
- `migrations/20260613014000_create_agreements_signatures_reviews_trust.sql` — `trust_scores` table.
- `migrations/20260615120000_review_indexes_constraints_and_scopes.sql` — scope seeding pattern.
- `trust-score.md` — scoring formulas.
- `party-guide.md` — Party entity rules.
- `review.md` — admin-review pattern reference.
- `AGENTS.md` — project conventions.
