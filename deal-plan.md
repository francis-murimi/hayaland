# Deal Execution Action Plan (DEAP) — Hayaland 3-Party Deals

> **Status:** Ready for implementation  
> **Sources:** `hayaland` Rust codebase (Actix Web + sqlx + PostgreSQL, hexagonal architecture), `3partydeal.pdf` Software Design Document, `hayaland-deal-plan.md`, `party-guide.md`

---

## 1. Goal

Implement a **3-Party Deal** capability in Hayaland that lets any combination of Supplier, Consumer, and Enhancer create, negotiate, commit to, execute, and complete a value exchange — while enforcing the platform's core **Win-Win-Win** principle.

All value exchange on the platform is tracked in **platform points**, not real money. Each party has a wallet that records point balances and every transaction. These platform transactions mirror physical transactions that parties perform outside the platform in real life (e.g., bank transfers, cash exchanges, in-kind deliveries). Every transaction must be approved by each involved party before it is marked as verified/complete.

This plan is the executable distillation of the larger design documents. It assumes the existing Hayaland codebase (users, JWT auth, role scopes, email, parties) is in place and treats the **Deal domain** as the next bounded context to build.

---

## 2. Current State of Hayaland

The codebase already provides a solid foundation:

| Layer | What's Ready |
|-------|-------------|
| **Domain** | `User`, `Party`, `PartyRole`/`DealRole` (Supplier/Consumer/Enhancer), `UserPartyMembership`, value objects, repository ports, `DomainError` |
| **Application** | User CRUD, auth, email verification, password reset, party CRUD, role management, search/nearby |
| **Infrastructure** | PostgreSQL repositories, Argon2, JWT, SMTP email queue, config, migrations, PostGIS geo queries |
| **API** | Actix Web routes, auth middleware with scope checks, `AuthContext`, structured errors |

What is **not yet implemented**: `Deal`, `DealParticipation`, `Resource`, `Need`, `Enhancement`, `Term`, `ValueDistribution`, `Milestone`, `Agreement`, `Signature`, `Review`, `TrustScore`, `Match`, `PlatformWallet`, `Transaction`, `Dispute`, and the full deal state machine.

---

## 3. Deal Domain at a Glance

A deal always involves exactly three distinct parties, each playing one role:

- **Supplier** — provides an underutilized resource (land, vehicle, building, machinery, materials, data).
- **Consumer** — wants a specific output/product/service.
- **Enhancer** — provides the expertise/input that makes Supplier → Consumer feasible.

Any role can initiate a deal. The platform validates that **all three parties receive positive net value** before commitment. The platform fee is configured **per deal** during value distribution, not globally.

### 3.1 13-State Deal Lifecycle

```text
[*] → DRAFT → SUGGESTED → PENDING_REVIEW → NEGOTIATING → TERMS_LOCKED → COMMITTED → EXECUTING → COMPLETED
              ↓                ↓                ↓              ↓              ↓            ↓
         CANCELLED        CANCELLED        CANCELLED      CANCELLED     CANCELLED      DISPUTED
         EXPIRED          EXPIRED          AWAITING_PARTY  (renegotiate)  (3-day prep)   ↓
                                           ON_HOLD                                      RESOLVED → COMPLETED/CANCELLED
```

Terminal states: `COMPLETED`, `CANCELLED`, `EXPIRED`.

### 3.2 Win-Win-Win Validation Gate

Before commitment, a deal must satisfy:

| Gate | Rule |
|------|------|
| **Economic value** | Each party's net gain > 0; total deal value ≥ platform minimum; no cost exceeds 80% of budget/capacity. |
| **Balance** | No party captures > 70% of total surplus; each party ≥ 10% (warn); Gini-inspired variance ≤ threshold. |
| **Feasibility** | Resource available; need matches supply; enhancer capability fits. |
| **Risk** | No party has > 10 active executing deals; no 3-strike cancelled/disputed pattern. |
| **Compliance** | Parties verified (or flagged); category not restricted; jurisdiction OK. |

Validation produces a fairness score (0–100) and per-party feedback.

### 3.3 Value Distribution Models

All amounts below are denominated in **platform points**, which represent value exchanged and recorded on the platform. The actual settlement happens outside the platform; the platform ledger only tracks points, approvals, and obligations.

1. **Fixed Price** — consumer commits a fixed number of points; per-deal platform fee deducted; remainder split between supplier and enhancer.
2. **Revenue Share** — output value is jointly estimated in points; proceeds distributed by percentage after per-deal platform fee and reserve.
3. **Cost-Plus** — transparent cost stack in points + agreed margins.
4. **Barter/Exchange** — non-monetary exchange with platform point valuation guidance.
5. **Hybrid** — combination of upfront, milestone, and revenue-share components, all in points.

The MVP starts with **Fixed Price**, followed by **Cost-Plus** and **Revenue Share**.

---

## 4. Implementation Strategy

### 4.1 Architecture principles

- Keep the monolith. Do not introduce microservices yet.
- Add a new **Deal bounded context** inside the existing crate structure.
- Reuse existing `Party`, `User`, auth, scopes, and repository patterns.
- Introduce ports (`EventPublisher`, `PointsLedger`/`EscrowService`, `MatchingEngine`) with in-memory/ledger-only implementations first.
- Payments are **point-based**: wallets store points, transactions record point movements, and each transaction must be approved by every party involved before it becomes verified/complete.
- The **platform fee percentage is set per deal** during value distribution.
- **Deal timelines are set per deal** (`expected_start_date`, `expected_end_date`, milestone due dates).
- Add a **Deal Manager** platform role with scope `admin:deals` for users who oversee and moderate deals (distinct from `admin` super-admins).
- All new tables are additive; no breaking changes to existing `users`, `role_definitions`, `parties`, etc.

### 4.2 Multiplicity rules (hard invariants)

- A `Deal` has exactly three `DealParticipation` records, one per `DealRole`.
- A single `Party` may participate in many deals and may play different roles in different deals.
- Within one deal, a `Party` may hold **at most one role**, and the three participating parties must be distinct.
- The initiator party must be one of the three participations with `is_initiator = true`.

### 4.3 Deal Visibility & Access Control

**Deal details are private.** A deal is never publicly visible or accessible to arbitrary authenticated users. Access is restricted to:

1. **Party members** of the three participating parties — any user who is an active member (`OWNER`, `ADMIN`, `MEMBER`, or `OBSERVER`) of a party participating in the deal may view the deal.
2. **Deal Managers / Admins** — users holding the `admin:deals` or `admin:*` scope may view and manage any deal for moderation, support, and oversight.

Access checks are enforced at the application and API layers. Public fields such as anonymized search results or public party profiles do not include deal details.

### 4.4 Acting as a Party

Deal endpoints require the caller to select which Party they act as via the `X-Party-ID` header. If the user belongs to exactly one active party, the system may default to it; otherwise the header is required.

### 4.5 Points & Transaction Approval Model

The platform does **not** move real money. Instead, it operates an internal **points ledger**.

- **Points** represent deal value and obligations on the platform.
- Each `Party` has one `PlatformWallet` storing `balance`, `escrow_balance`, and `pending_balance`.
- A `Transaction` records every point movement: deposits, withdrawals, escrow holds, escrow releases, platform fees, and adjustments.
- Each transaction **mirrors a physical settlement** performed outside the platform (bank transfer, cash, in-kind delivery, etc.). The `payment_method` and `external_reference` fields capture the real-world counterpart.
- **Multi-party approval**: every transaction with `requires_approval = true` must be approved by each party referenced in `from_party_id` and `to_party_id`.
  - `PENDING` → when all required parties approve → `VERIFIED`/`COMPLETE`.
  - If any required party rejects → `REJECTED`; the point movement is not applied.
- The **platform fee percentage is set per deal** inside `ValueDistribution`.
- **Deal timelines are set per deal** (`expected_start_date`, `expected_end_date`, `timeline` JSONB, milestone due dates).

### 4.6 Deal Manager Role

A new platform role scope `admin:deals` allows designated users (deal managers/moderators) to oversee deals without having full super-admin privileges.

Capabilities:

- List all deals (`GET /api/v1/admin/deals`).
- Get any deal including private negotiation details (`GET /api/v1/admin/deals/{id}`).
- Suspend or resume a deal (`POST /api/v1/admin/deals/{id}/actions/suspend|resume`).
- Force a state transition when required (`POST /api/v1/admin/deals/{id}/actions/transition`).
- View deal audit history.

All deal-manager mutations are logged with admin user ID, reason, and before/after snapshot.

---

## 5. Phased Milestones

### Phase 0 — Foundation (Weeks 1–2)

**Goal:** The existing party system is production-ready for deal participation.

- [ ] Ensure `Party`, `PartyRole`, `UserPartyMembership`, categories, and geo search are complete and tested.
- [ ] Seed initial category taxonomy (Agriculture, Real Estate, Transportation, Manufacturing, Technology, plus resource/need/enhancement sub-types).
- [ ] Add deal-specific scopes to `role_definitions`:
  - `deals:read`, `deals:write`, `deals:transition`
  - `terms:negotiate`
  - `payments:read`, `payments:write`
  - `admin:deals` (for deal managers/moderators)
- [ ] Add `X-Party-ID` resolution to `AuthContext`.
- [ ] Add admin deal-management API routes (skeleton, fully wired in later phases):
  - `GET /api/v1/admin/deals`
  - `GET /api/v1/admin/deals/{id}`
  - `POST /api/v1/admin/deals/{id}/actions/suspend`
  - `POST /api/v1/admin/deals/{id}/actions/resume`
  - `POST /api/v1/admin/deals/{id}/actions/transition`

### Phase 1 — Deal Lifecycle MVP (Weeks 3–6)

**Goal:** A deal can be drafted, submitted, negotiated, locked, committed, and completed.

- [ ] **Domain**: `Deal`, `DealStatus`, `DealParticipation`, `ParticipationStatus`, `DealRole`.
- [ ] **Domain**: `Resource`, `Need`, `Enhancement` as child entities of a deal.
- [ ] **Domain service**: `DealStateMachine` with explicit allowed transitions and actor/timeout rules.
- [ ] **Application use cases**: `CreateDraftDeal`, `GetDeal`, `ListDeals`, `UpdateDraftDeal`, `SubmitDeal`, `ExecuteDealTransition`, `ListDealHistory`.
- [ ] Enforce deal visibility: `GetDeal`, `ListDeals`, and deal-child endpoints must only return deals where the caller is a member of a participating party or has `admin:deals`/`admin:*`.
- [ ] **Infrastructure**: migrations and Postgres repositories for `deals`, `deal_participations`, `resources`, `needs`, `enhancements`, `deal_history`.
- [ ] **API routes**:
  - `POST /api/v1/deals`
  - `GET /api/v1/deals`
  - `GET /api/v1/deals/{id}`
  - `PUT|PATCH /api/v1/deals/{id}`
  - `GET /api/v1/deals/{id}/history`
  - `POST /api/v1/deals/{id}/transitions`
  - `GET /api/v1/deals/{id}/transitions`
- [ ] **Background worker**: expire drafts/suggested/pending_review/negotiating/on-hold deals per timeout rules.

### Phase 2 — Negotiation & Terms (Weeks 7–8)

**Goal:** Parties can negotiate clauses before locking terms.

- [ ] **Domain**: `Term`, `TermType`, `TermStatus`, `TermVersion`.
- [ ] **Application use cases**: `ProposeTerm`, `CounterTerm`, `AcceptTerm`, `RejectTerm`, `ListTerms`.
- [ ] **API routes**:
  - `GET|POST /api/v1/deals/{id}/terms`
  - `POST /api/v1/deals/{id}/terms/{termId}/accept`
  - `POST /api/v1/deals/{id}/terms/{termId}/reject`
  - `POST /api/v1/deals/{id}/terms/{termId}/counter`

### Phase 3 — Value Distribution & Win-Win-Win Validation (Weeks 9–10)

**Goal:** Enforce positive net value for all parties and fair distribution.

- [ ] **Domain**: `ValueDistribution`, `DistributionModel`, `PaymentScheduleEntry`.
- [ ] **Domain service**: `WinWinWinValidator` (PVE normalization, critical/warning rules, fairness score).
- [ ] **Application use cases**: `SetValueDistribution`, `ValidateDeal`.
- [ ] **API routes**:
  - `GET|PUT /api/v1/deals/{id}/value-distribution`
  - `POST /api/v1/deals/{id}/validate`
- [ ] Integrate validation into `SubmitDeal`, `LockTerms`, and `Commit` transitions.

### Phase 4 — Agreement, Execution & Settlement (Weeks 11–13)

**Goal:** Convert locked terms into a signed agreement with milestone-based point releases and multi-party approved transaction records.

- [ ] **Domain**: `Agreement`, `Signature`, `SignatureType`.
- [ ] **Domain**: `Milestone`, `MilestoneStatus`, `VerificationMethod`.
- [ ] **Domain**: `PlatformWallet`, `Transaction`, `TransactionType`, `TransactionStatus`, `TransactionApproval`.
- [ ] **Ports**: `PointsLedger`/`EscrowService` with point-based ledger implementation.
- [ ] **Application use cases**:
  - `GenerateAgreement`, `SignAgreement`
  - `CreateMilestone`, `UpdateMilestone`, `VerifyMilestone`
  - `RecordTransaction`, `ApproveTransaction`, `ListPendingTransactionApprovals`
  - `DepositPoints`, `WithdrawPoints`, `ReleasePoints`
- [ ] **Transaction approval flow**:
  - A transaction is created in `PENDING` status when a point movement is initiated.
  - Every party referenced in `from_party_id` and `to_party_id` must approve.
  - Once all involved parties approve, status moves to `VERIFIED`/`COMPLETE`.
  - If any party rejects, status moves to `REJECTED` and the point movement is not applied.
- [ ] **API routes**:
  - `GET|POST /api/v1/deals/{id}/agreement`
  - `POST /api/v1/deals/{id}/agreement/sign`
  - `GET|POST /api/v1/deals/{id}/milestones`
  - `PATCH /api/v1/deals/{id}/milestones/{mId}`
  - `POST /api/v1/deals/{id}/milestones/{mId}/verify`
  - `GET /api/v1/payments/wallets/me`
  - `POST /api/v1/payments/deposit`
  - `POST /api/v1/payments/withdraw`
  - `GET /api/v1/payments/transactions/pending-approvals`
  - `POST /api/v1/payments/transactions/{txnId}/approve`
  - `POST /api/v1/payments/transactions/{txnId}/reject`

### Phase 5 — Trust, Reviews & Matching (Weeks 14–16)

**Goal:** Help parties find each other and build reputation.

- [ ] **Domain**: `Review`, `TrustScore`.
- [ ] **Port**: `MatchingEngine` with deterministic SQL+Rust scoring.
- [ ] **Application use cases**: `SubmitReview`, `GetTrustScore`, `RecalculateTrustScore`, `FindMatches`, `RespondToMatch`.
- [ ] **API routes**:
  - `POST /api/v1/deals/{id}/reviews`
  - `GET /api/v1/trust/party/{partyId}`
  - `GET /api/v1/matches`
  - `POST /api/v1/matches/{id}/respond`

### Phase 6 — Future Capabilities (Post-MVP)

- [ ] Multi-threaded parallel negotiation with term dimension locking and ZOPA visualization.
- [ ] Three-tier dispute resolution workflow.
- [ ] Party groups with governance/voting for deal actions.
- [ ] Optional: bridge points to external payment providers or tokenized settlement (out of MVP scope).
- [ ] Event sourcing for deal lifecycle audit trail.
- [ ] GraphQL/read-model projections (CQRS).
- [ ] Extraction of bounded contexts into services behind HTTP/gRPC adapters.

---

## 6. Crate-by-Crate Additions

### `crates/domain`

New entities and services:

```text
entities/
  category.rs           (already exists, extend as needed)
  deal.rs
  deal_participation.rs
  resource.rs
  need.rs
  enhancement.rs
  term.rs
  value_distribution.rs
  milestone.rs
  agreement.rs
  signature.rs
  review.rs
  trust_score.rs
  match_suggestion.rs
  wallet.rs
  transaction.rs
  dispute.rs
repositories/
  deal_repository.rs
  term_repository.rs
  value_distribution_repository.rs
  milestone_repository.rs
  agreement_repository.rs
  review_repository.rs
  trust_score_repository.rs
  match_repository.rs
  wallet_repository.rs
services/
  deal_state_machine.rs
  win_win_win_validator.rs
errors.rs               (extend DomainError)
```

### `crates/application`

New modules following the existing use-case pattern (command/result structs + `execute` method):

```text
deals/
  create_deal.rs
  get_deal.rs
  list_deals.rs
  update_deal.rs
  submit_deal.rs
  execute_transition.rs
  list_deal_history.rs
terms/
  propose_term.rs
  counter_term.rs
  accept_term.rs
  reject_term.rs
  list_terms.rs
value_distribution/
  set_value_distribution.rs
  validate_deal.rs
agreements/
  generate_agreement.rs
  sign_agreement.rs
milestones/
  create_milestone.rs
  update_milestone.rs
  verify_milestone.rs
payments/
  deposit_points.rs
  withdraw_points.rs
  release_points.rs
  record_transaction.rs
  approve_transaction.rs
  list_pending_transaction_approvals.rs
admin_deals/
  list_deals.rs
  get_deal.rs
  suspend_deal.rs
  force_transition.rs
matching/
  find_matches.rs
  respond_to_match.rs
reviews/
  submit_review.rs
trust/
  get_trust_score.rs
  recalculate_trust_score.rs
events/
  mod.rs
  event_publisher.rs     (outbound port)
```

New outbound ports:

- `EventPublisher` — publish domain events (in-memory broadcast channel in MVP).
- `PointsLedger` / `EscrowService` — point deposit, hold, release, refund; records mirror external physical settlements.
- `TransactionApprover` — enforces that all parties involved in a transaction approve it before completion.
- `MatchingEngine` — find compatible triplets.
- `DocumentRenderer` — generate agreement text from locked terms.

### `crates/infrastructure`

```text
repositories/
  postgres_deal_repository.rs
  postgres_term_repository.rs
  postgres_value_distribution_repository.rs
  postgres_milestone_repository.rs
  postgres_agreement_repository.rs
  postgres_review_repository.rs
  postgres_trust_score_repository.rs
  postgres_match_repository.rs
  postgres_wallet_repository.rs
matching/
  sql_matching_engine.rs
payments/
  points_ledger_service.rs
events/
  in_memory_event_publisher.rs
```

All repositories use `sqlx::query!` macros. Run `cargo sqlx prepare --workspace` after each migration batch and commit `.sqlx/` metadata.

### `crates/api`

```text
routes/
  deals.rs
  terms.rs
  value_distribution.rs
  agreements.rs
  milestones.rs
  payments.rs
  matching.rs
  reviews.rs
  trust.rs
  admin_deals.rs
handlers/
  deals/
  terms/
  ...
dto/
  deals.rs
  terms.rs
  ...
```

Middleware helpers:

- `require_deal_participant(ctx, deal_id, repo)` — user is a member of one of the three participating parties.
- `require_deal_role(ctx, deal_id, role, repo)` — user acts as the party holding the specified role in the deal.
- `require_deal_visibility(ctx, deal_id, repo)` — user is either a participating party member or holds `admin:deals`/`admin:*`; used for `GET` operations.
- `require_deal_admin(ctx)` — user holds `admin:deals` or `admin:*`.
- `resolve_acting_party(ctx)` from `X-Party-ID`

---

## 7. Core Database Schema

All tables are additive. Existing tables are untouched.

### 7.1 Deals and participations

```sql
CREATE TABLE deals (
    id UUID PRIMARY KEY,
    deal_reference TEXT NOT NULL UNIQUE,
    deal_title TEXT NOT NULL,
    deal_description TEXT,
    domain_category_id UUID NOT NULL REFERENCES categories(id),
    initiator_party_id UUID NOT NULL REFERENCES parties(id),
    initiator_role TEXT NOT NULL CHECK (initiator_role IN ('SUPPLIER','CONSUMER','ENHANCER')),
    deal_status TEXT NOT NULL DEFAULT 'DRAFT',
    expected_start_date DATE,
    expected_end_date DATE,
    actual_start_date DATE,
    actual_end_date DATE,
    timeline JSONB, -- per-deal timeline: key milestones/dates negotiated by parties
    location_geo GEOGRAPHY(POINT),
    location_address JSONB,
    total_deal_value DECIMAL, -- in platform points
    currency TEXT DEFAULT 'POINTS',
    platform_fee_percentage DECIMAL NOT NULL DEFAULT 0, -- set per deal
    platform_fee_amount DECIMAL NOT NULL DEFAULT 0, -- in platform points
    win_win_win_validated BOOLEAN NOT NULL DEFAULT false,
    validation_checked_at TIMESTAMPTZ,
    validation_score DECIMAL,
    validation_result JSONB,
    is_public BOOLEAN NOT NULL DEFAULT false, -- only affects match/discovery metadata, never exposes deal details
    current_state_entered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE deal_participations (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    party_id UUID NOT NULL REFERENCES parties(id),
    role TEXT NOT NULL CHECK (role IN ('SUPPLIER','CONSUMER','ENHANCER')),
    participation_status TEXT NOT NULL DEFAULT 'INVITED' CHECK (participation_status IN ('INVITED','PENDING','ACCEPTED','DECLINED','WITHDRAWN')),
    is_initiator BOOLEAN NOT NULL DEFAULT false,
    value_share_percentage DECIMAL,
    value_share_amount DECIMAL,
    invited_at TIMESTAMPTZ,
    responded_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (deal_id, role),
    UNIQUE (deal_id, party_id)
);

CREATE TABLE deal_history (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    actor_party_id UUID REFERENCES parties(id),
    details JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 7.2 Child deal tables

```sql
CREATE TABLE resources (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    supplier_party_id UUID NOT NULL REFERENCES parties(id),
    resource_type_id UUID NOT NULL REFERENCES categories(id),
    resource_name TEXT NOT NULL,
    description TEXT,
    quantity DECIMAL NOT NULL,
    quantity_unit TEXT NOT NULL,
    condition TEXT,
    location_geo GEOGRAPHY(POINT),
    availability_start DATE,
    availability_end DATE,
    document_urls TEXT[],
    opportunity_cost DECIMAL,
    verified_by_platform BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE needs (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    consumer_party_id UUID NOT NULL REFERENCES parties(id),
    need_category_id UUID NOT NULL REFERENCES categories(id),
    need_description TEXT NOT NULL,
    required_quantity DECIMAL NOT NULL,
    quantity_unit TEXT NOT NULL,
    quality_requirements TEXT,
    required_by_date DATE,
    max_budget DECIMAL,
    budget_currency TEXT,
    estimated_fulfillment_value DECIMAL,
    acceptable_variants TEXT,
    priority TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE enhancements (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    enhancer_party_id UUID NOT NULL REFERENCES parties(id),
    enhancement_type_id UUID NOT NULL REFERENCES categories(id),
    enhancement_name TEXT NOT NULL,
    description TEXT,
    input_quantity DECIMAL,
    quantity_unit TEXT,
    estimated_input_cost DECIMAL,
    service_duration_hours DECIMAL,
    estimated_completion_days INTEGER,
    deliverables TEXT,
    prerequisites TEXT,
    is_complete BOOLEAN NOT NULL DEFAULT false,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE terms (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    proposed_by_party_id UUID NOT NULL REFERENCES parties(id),
    term_type TEXT NOT NULL,
    term_name TEXT NOT NULL,
    description TEXT NOT NULL,
    negotiation_status TEXT NOT NULL DEFAULT 'PROPOSED',
    parent_term_id UUID REFERENCES terms(id),
    version INTEGER NOT NULL DEFAULT 1,
    proposed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ,
    is_mandatory BOOLEAN NOT NULL DEFAULT false,
    resolution TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE value_distributions (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL UNIQUE REFERENCES deals(id) ON DELETE CASCADE,
    total_value DECIMAL NOT NULL,
    currency TEXT NOT NULL DEFAULT 'POINTS',
    distribution_model TEXT NOT NULL,
    supplier_share_percentage DECIMAL NOT NULL,
    supplier_share_amount DECIMAL NOT NULL,
    consumer_cost_percentage DECIMAL NOT NULL,
    consumer_cost_amount DECIMAL NOT NULL,
    enhancer_share_percentage DECIMAL NOT NULL,
    enhancer_share_amount DECIMAL NOT NULL,
    platform_fee_percentage DECIMAL NOT NULL,
    platform_fee_amount DECIMAL NOT NULL,
    payment_schedule JSONB NOT NULL DEFAULT '[]',
    win_win_win_score DECIMAL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE milestones (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    milestone_name TEXT NOT NULL,
    description TEXT,
    assigned_to_party_id UUID REFERENCES parties(id),
    due_date DATE,
    completion_criteria TEXT NOT NULL,
    milestone_status TEXT NOT NULL DEFAULT 'PENDING',
    completion_percentage DECIMAL NOT NULL DEFAULT 0,
    payment_trigger_amount DECIMAL,
    completed_at TIMESTAMPTZ,
    verified_by_party_id UUID REFERENCES parties(id),
    display_order INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 7.3 Agreements, reviews, trust, matching, payments

```sql
CREATE TABLE agreements (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL UNIQUE REFERENCES deals(id) ON DELETE CASCADE,
    agreement_status TEXT NOT NULL DEFAULT 'DRAFT',
    agreement_text TEXT NOT NULL,
    governing_law TEXT,
    dispute_resolution TEXT,
    effective_date DATE,
    termination_date DATE,
    auto_renew BOOLEAN NOT NULL DEFAULT false,
    version INTEGER NOT NULL DEFAULT 1,
    digital_signature_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    executed_at TIMESTAMPTZ
);

CREATE TABLE signatures (
    id UUID PRIMARY KEY,
    agreement_id UUID NOT NULL REFERENCES agreements(id) ON DELETE CASCADE,
    party_id UUID NOT NULL REFERENCES parties(id),
    signed_by_user_id UUID NOT NULL REFERENCES users(id),
    signature_type TEXT NOT NULL,
    signature_data TEXT NOT NULL,
    ip_address TEXT,
    signed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agreement_id, party_id)
);

CREATE TABLE reviews (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    reviewer_party_id UUID NOT NULL REFERENCES parties(id),
    reviewed_party_id UUID NOT NULL REFERENCES parties(id),
    reviewed_role TEXT NOT NULL,
    overall_rating INTEGER NOT NULL CHECK (overall_rating BETWEEN 1 AND 5),
    communication_rating INTEGER CHECK (communication_rating BETWEEN 1 AND 5),
    reliability_rating INTEGER CHECK (reliability_rating BETWEEN 1 AND 5),
    quality_rating INTEGER CHECK (quality_rating BETWEEN 1 AND 5),
    timeliness_rating INTEGER CHECK (timeliness_rating BETWEEN 1 AND 5),
    review_text TEXT,
    is_verified BOOLEAN NOT NULL DEFAULT false,
    is_public BOOLEAN NOT NULL DEFAULT true,
    platform_response TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE trust_scores (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL UNIQUE REFERENCES parties(id) ON DELETE CASCADE,
    overall_score DECIMAL NOT NULL DEFAULT 0,
    as_supplier_score DECIMAL,
    as_consumer_score DECIMAL,
    as_enhancer_score DECIMAL,
    deals_completed_count INTEGER NOT NULL DEFAULT 0,
    deals_cancelled_count INTEGER NOT NULL DEFAULT 0,
    deals_disputed_count INTEGER NOT NULL DEFAULT 0,
    average_response_hours DECIMAL,
    profile_completeness DECIMAL NOT NULL DEFAULT 0,
    verification_level INTEGER NOT NULL DEFAULT 0,
    longevity_days INTEGER NOT NULL DEFAULT 0,
    calculation_formula JSONB NOT NULL DEFAULT '{}',
    last_calculated_at TIMESTAMPTZ,
    next_calculation_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE match_suggestions (
    id UUID PRIMARY KEY,
    supplier_party_id UUID NOT NULL REFERENCES parties(id),
    consumer_party_id UUID NOT NULL REFERENCES parties(id),
    enhancer_party_id UUID NOT NULL REFERENCES parties(id),
    match_status TEXT NOT NULL DEFAULT 'PENDING',
    match_score DECIMAL NOT NULL,
    match_reason TEXT,
    resource_category_id UUID REFERENCES categories(id),
    need_category_id UUID REFERENCES categories(id),
    enhancement_category_id UUID REFERENCES categories(id),
    suggested_deal_value DECIMAL,
    generated_by TEXT NOT NULL DEFAULT 'ALGORITHM',
    expires_at TIMESTAMPTZ,
    converted_deal_id UUID REFERENCES deals(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE platform_wallets (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL UNIQUE REFERENCES parties(id) ON DELETE CASCADE,
    balance DECIMAL NOT NULL DEFAULT 0, -- platform points
    escrow_balance DECIMAL NOT NULL DEFAULT 0, -- points held in escrow
    pending_balance DECIMAL NOT NULL DEFAULT 0, -- points awaiting approval
    total_deposited DECIMAL NOT NULL DEFAULT 0,
    total_withdrawn DECIMAL NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'POINTS',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE transactions (
    id UUID PRIMARY KEY,
    deal_id UUID REFERENCES deals(id),
    agreement_id UUID REFERENCES agreements(id),
    milestone_id UUID REFERENCES milestones(id),
    transaction_type TEXT NOT NULL, -- DEPOSIT, WITHDRAWAL, ESCROW_HOLD, ESCROW_RELEASE, FEE, ADJUSTMENT
    from_party_id UUID REFERENCES parties(id),
    to_party_id UUID REFERENCES parties(id),
    amount DECIMAL NOT NULL, -- platform points
    currency TEXT NOT NULL DEFAULT 'POINTS',
    description TEXT,
    status TEXT NOT NULL DEFAULT 'PENDING', -- PENDING, VERIFIED, COMPLETE, REJECTED
    payment_method TEXT, -- e.g., BANK_TRANSFER, CASH, IN_KIND, OTHER (mirrors external settlement)
    external_reference TEXT, -- reference to physical transaction outside platform
    requires_approval BOOLEAN NOT NULL DEFAULT true,
    approvals_required INTEGER NOT NULL DEFAULT 2,
    approvals_received INTEGER NOT NULL DEFAULT 0,
    executed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE transaction_approvals (
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

### 7.4 Indexes

```sql
CREATE INDEX idx_deals_status ON deals(deal_status);
CREATE INDEX idx_deals_initiator ON deals(initiator_party_id);
CREATE INDEX idx_deals_domain ON deals(domain_category_id);
CREATE INDEX idx_deals_geo ON deals USING GIST(location_geo);
CREATE INDEX idx_participations_party ON deal_participations(party_id);
CREATE INDEX idx_participations_deal ON deal_participations(deal_id);
CREATE INDEX idx_deal_history_deal ON deal_history(deal_id, created_at DESC);
CREATE INDEX idx_match_suggestions_supplier ON match_suggestions(supplier_party_id);
CREATE INDEX idx_match_suggestions_consumer ON match_suggestions(consumer_party_id);
CREATE INDEX idx_match_suggestions_enhancer ON match_suggestions(enhancer_party_id);
CREATE INDEX idx_transactions_deal ON transactions(deal_id);
CREATE INDEX idx_transactions_status ON transactions(status);
CREATE INDEX idx_transaction_approvals_txn ON transaction_approvals(transaction_id);
CREATE INDEX idx_transaction_approvals_party ON transaction_approvals(party_id);
```

---

## 8. State Machine Rules

| Transition | Trigger | Validation |
|---|---|---|
| `DRAFT → SUGGESTED` | Initiator submits proposal | All participations present; basic fields valid; at least resource/need/enhancement per initiator role. |
| `SUGGESTED → PENDING_REVIEW` | System confirms all parties exist and are active | Invited parties active/verified; no duplicate active deals. |
| `PENDING_REVIEW → NEGOTIATING` | All parties acknowledge | Each party reviewed proposal. |
| `NEGOTIATING → TERMS_LOCKED` | All parties accept current terms | No pending counter-proposals; value distribution set. |
| `TERMS_LOCKED → COMMITTED` | All parties sign | Win-Win-Win validation passes; agreement generated; per-deal platform fee set; point escrow allocated/committed. |
| `COMMITTED → EXECUTING` | 3-day prep elapsed OR unanimous begin-early | Point escrow committed; milestones enabled; deal timeline active. |
| `EXECUTING → COMPLETED` | All milestones verified | Point releases recorded as transactions; all involved parties approve each transaction; reviews requested. |
| `EXECUTING → DISPUTED` | Any party raises dispute | Evidence uploaded; good-faith deposit paid. |

Timeouts:

| State | Timeout |
|---|---|
| `DRAFT` | 7 days → auto-delete |
| `SUGGESTED` | 14 days → EXPIRED |
| `PENDING_REVIEW` | 14 days → EXPIRED |
| `NEGOTIATING` | 30 days → ON_HOLD |
| `AWAITING_PARTY` | 14 days → ON_HOLD |
| `TERMS_LOCKED` | 14 days → CANCELLED |
| `COMMITTED` | 3 days → EXECUTING |
| `ON_HOLD` | 30 days → CANCELLED |
| `DISPUTED` | 14 days → escalate to arbitration |

---

## 9. Win-Win-Win Validator Spec

Location: `crates/domain/src/services/win_win_win_validator.rs`

Inputs:

- `ValueDistribution`
- `Resource` (opportunity cost)
- `Need` (max budget, estimated fulfillment value)
- `Enhancement` (estimated input cost)
- Party snapshots (trust score, verification level, active deal count)
- Per-deal `platform_fee_percentage` (set during value distribution)
- Domain config (min deal size, discount rate, share thresholds)

Algorithm:

1. Normalize all value types to **Present Value Equivalent (PVE)** in base points.
2. Run critical rules (BLOCK if violated):
   - All party gains > 0.
   - No share > 70%.
   - Consumer cost < independent sourcing × 0.95 (or ≤ 1.05 with additional value).
   - Deal value ≥ min deal size.
3. Run warning rules (FLAG + require acknowledgment):
   - Any share < 10%.
   - Enhancer value/cost-added outside 50–150%.
   - Risk ratio between max/min party > 3.0.
4. Compute fairness score (0–100):
   - Absolute gain: 25%
   - Proportional fairness (variance penalty): 30%
   - Market benchmark: 25%
   - Opportunity cost: 20%
5. Return `ValidationResult` with score, status, violations, warnings, and per-party feedback.

Tiers:

- 90–100: Excellent
- 70–89: Good
- 50–69: Fair (warn)
- < 50: Poor (revise)

---

## 10. Testing Strategy

- **Domain unit tests**: invariants for `Deal`, `ValueDistribution`, `Term`; state machine transition matrix; validator sample cases.
- **Application tests**: fake repositories in `test_helpers.rs`; use-case tests for create/submit/transition/validate/sign/verify.
- **Infrastructure tests**: Postgres repository integration tests using `sqlx::test`.
- **API tests**: happy-path and error-path scenarios for deal creation, transitions, term negotiation, validation. Include visibility tests: non-participants and non-admin users receive `403 Forbidden` for deal detail endpoints.
- **CI**: keep `cargo fmt --check && cargo clippy -- -D warnings`, `cargo test`, and `cargo sqlx prepare --workspace --check` passing.

---

## 11. Migration Order

Create one migration file per logical addition:

1. `seed_categories_and_update_role_scopes.sql`
2. `create_deals_table.sql`
3. `create_deal_participations_table.sql`
4. `create_deal_history_table.sql`
5. `create_resources_table.sql`
6. `create_needs_table.sql`
7. `create_enhancements_table.sql`
8. `create_terms_table.sql`
9. `create_value_distributions_table.sql`
10. `create_milestones_table.sql`
11. `create_agreements_table.sql`
12. `create_signatures_table.sql`
13. `create_reviews_table.sql`
14. `create_trust_scores_table.sql`
15. `create_match_suggestions_table.sql`
16. `create_platform_wallets_table.sql`
17. `create_transactions_table.sql`
18. `create_transaction_approvals_table.sql`
19. `add_deal_indexes.sql`

All migrations must be idempotent and backwards-compatible.

---

## 12. Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Schema grows large; `.sqlx/` metadata hard to maintain | Add tables incrementally; run `cargo sqlx prepare` after each milestone. |
| Market benchmarks not available | Stub `MarketBenchmarkProvider` port; start with configurable defaults and party-provided estimates. |
| Discrepancies between platform points and physical settlement | Require multi-party transaction approval; allow attachments/external reference IDs; provide dispute workflow. |
| `X-Party-ID` adds request complexity | Default to sole active party; require header only when user has multiple parties. |
| Monolith becomes too large | Enforce strict module boundaries per bounded context; use internal event publisher. |

---

## 13. Success Criteria

- [ ] User can register, verify email, create a party, and assign S/C/E role profiles.
- [ ] Any role can initiate a deal and invite the other two parties.
- [ ] Deal details are visible only to members of participating parties and users with `admin:deals`/`admin:*`; deals are not publicly accessible.
- [ ] Deal flows through `DRAFT → SUGGESTED → PENDING_REVIEW → NEGOTIATING → TERMS_LOCKED → COMMITTED → EXECUTING → COMPLETED`.
- [ ] Parties can propose/accept/reject terms and set a value distribution.
- [ ] Win-Win-Win validation blocks critical violations and flags warnings.
- [ ] Agreement is generated and signed by all parties.
- [ ] Per-deal platform fee and timeline are configured during value distribution.
- [ ] Milestones are tracked and verified; point releases are recorded and approved by all involved parties before completion.
- [ ] Post-deal reviews update trust scores.
- [ ] Basic matching returns ranked suggestions.
- [ ] Deal managers with `admin:deals` can list, view, suspend, and force-transition deals.
- [ ] All existing user/auth/party tests continue to pass.

---

## 14. Immediate Next Steps

1. Review and approve this DEAP.
2. Create a feature branch.
3. Implement Phase 0 (categories, scopes, `X-Party-ID` resolution).
4. Add domain/application tests for Phase 1 before writing infrastructure/API code.
5. Proceed incrementally through phases, keeping migrations and `.sqlx/` metadata current.

---

## 15. Glossary

| Term | Meaning |
|---|---|
| **Party** | Business identity that participates in deals (individual, organization, or group). Distinct from `User`. |
| **Deal** | A 3-party arrangement with one Supplier, one Consumer, and one Enhancer. |
| **DealRole** | `SUPPLIER`, `CONSUMER`, or `ENHANCER` within a specific deal. |
| **Participation** | The link between a Party and a Deal in a specific role. |
| **Term** | A negotiable clause with versioning. |
| **ValueDistribution** | Allocation of total deal value among parties and platform. |
| **Win-Win-Win Validation** | Domain service ensuring all three parties achieve positive net value with fair distribution. |
| **Escrow** | Internal point ledger holding consumer-committed points until milestones are verified and transactions are approved. |
| **TrustScore** | Composite reputation metric per party derived from reviews and history. |
| **PVE** | Present Value Equivalent — common point-based unit for validation. |
| **Points** | Platform-internal unit of value; transactions mirror external physical settlements. |
| **Transaction Approval** | Approval by every party involved in a transaction before it is marked verified/complete. |
| **Deal Manager** | Platform user with `admin:deals` scope who can oversee and moderate deals. |
