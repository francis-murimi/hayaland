# Hayaland 3-Party Deal Platform — Implementation Plan

> **Status:** Draft plan — no code changes yet.  
> **Based on:** Current `hayaland` codebase (Rust workspace, Actix Web + sqlx + PostgreSQL, hexagonal architecture) and the `3partydeal.pdf` Software Design Document.

---

## 1. Executive Summary

`hayaland` today is a clean, hexagonal Rust web application with user/identity management, role-based scopes, JWT authentication, email verification, and password reset. It provides a solid foundation but has **no domain entities, repositories, or API surface for deals, parties, or value exchange**.

The `3partydeal.pdf` specification describes a **domain-agnostic, triadic marketplace** connecting:

- **Suppliers** (idle resources/assets)
- **Consumers** (desired outputs/needs)
- **Enhancers** (enabling expertise/services)

with a 13-state deal lifecycle, Win-Win-Win validation, value distribution, escrow/settlement, matching, trust scoring, and dispute resolution.

This plan proposes incrementally building the 3-Party Deal capability **inside the existing `hayaland` architecture**, reusing the current user/identity/scope infrastructure and extending it with new domain aggregates, use cases, repositories, and HTTP routes. The goal is an MVP that supports deal creation, negotiation, commitment, and completion in a single Rust monolith, with clear extension points for future microservices, event sourcing, or payment provider integrations.

---

## 2. Current Codebase Assessment

### 2.1 Architecture

| Crate | Responsibility | Current contents |
|---|---|---|
| `crates/domain` | Entities, value objects, repository ports, domain errors | `User`, `Role`, `EmailVerification`, `PasswordResetToken`, plus repository traits |
| `crates/application` | Use cases / application services, DTOs, outbound ports | User CRUD, auth, email, password reset, role scope management |
| `crates/infrastructure` | Implementations of domain/application ports | Postgres repositories, Argon2 hasher, JWT service, SMTP email, config, migrations |
| `crates/api` | Actix Web wiring | Routes, handlers, DTOs, auth middleware, `AppState` |

Dependency direction is already correct for the new domain: `api → application → domain` and `api → infrastructure → application/domain`.

### 2.2 Existing patterns to reuse

- **Aggregate roots** with public fields and factory methods (`User::new`).
- **Value objects** (`Email`, `Username`, `PasswordHash`) using `validator` and returning `DomainError`.
- **Repository ports** defined in `domain` as `#[async_trait]` traits returning `DomainError`.
- **Postgres implementations** using `sqlx::query!` macros with offline query metadata stored in `.sqlx/`.
- **Application use cases** as structs holding `Arc<dyn RepositoryPort>` and exposing an `execute` method.
- **Application errors** (`ApplicationError`) with a `From<DomainError>` mapping.
- **API errors** (`ApiError`) implementing `actix_web::ResponseError` with JSON `{ code, message }` bodies.
- **JWT auth middleware** validating `Authorization: Bearer <token>` and injecting `AuthContext` (user_id, roles, scopes).
- **Scope-based authorization** (`users:read`, `users:write`, `users:admin`) checked in handlers.
- **Email queue** (`EmailQueue` port) with an in-memory implementation and background worker.
- **Migrations** in `migrations/` using `sqlx migrate` and idempotent SQL.

### 2.3 Gaps relative to the 3-party concept

- No `Party`, `Deal`, `Resource`, `Need`, `Enhancement`, `Term`, `Milestone`, `Agreement`, `Transaction`, `Review`, `TrustScore`, `Category`, `Match`, `Dispute`, or `PlatformWallet` entities.
- No state machine for deal lifecycle.
- No value distribution or Win-Win-Win validation engine.
- No matching/discovery logic.
- No escrow or payment abstraction.
- No party-group governance model.
- The current `Role`/`role_definitions` model is platform-wide (user/admin scopes), not per-deal roles (Supplier/Consumer/Enhancer).

### 2.4 Strategic constraint: monolith-first

The PDF proposes a 16-service microservices architecture, event sourcing, CQRS, and sagas. The current `hayaland` codebase is a **single monolith**. This plan intentionally keeps the first several milestones as a monolith to:

- Leverage the existing working build/test/CI pipeline.
- Avoid operational complexity before product/market fit.
- Define clear aggregate boundaries inside one deployable unit so that future extraction is mechanical.

Each bounded context is still modeled as a separate module/domain aggregate, and inter-aggregate communication goes through explicit application ports. When scale or team size demands it, any context can be extracted behind an HTTP/gRPC client adapter.

---

## 3. Concept Summary (from 3partydeal.pdf)

### 3.1 Core economic model

A deal always involves exactly three parties, each playing one of:

- **Supplier** — provides an underutilized resource.
- **Consumer** — defines a need/output and provides payment/value.
- **Enhancer** — provides the enabling input/expertise that makes the Supplier→Consumer exchange feasible.

Any of the three roles can initiate a deal.

### 3.2 Deal lifecycle (13 states)

```
[*] → DRAFT → SUGGESTED → PENDING_REVIEW → NEGOTIATING → TERMS_LOCKED → COMMITTED → EXECUTING → COMPLETED
              ↓                ↓                ↓              ↓              ↓            ↓
         CANCELLED        CANCELLED        CANCELLED      CANCELLED     CANCELLED      DISPUTED
         EXPIRED          EXPIRED          AWAITING_PARTY  (renegotiate)  (3-day prep)   ↓
                                           ON_HOLD                                      RESOLVED → COMPLETED/CANCELLED
```

Terminal states: `COMPLETED`, `CANCELLED`, `EXPIRED`.

### 3.3 Win-Win-Win validation gate

Before commitment, a deal must satisfy:

- **Economic value check:** each party has net gain > 0; no cost exceeds 80% of budget/capacity; total deal value ≥ platform minimum.
- **Balance check:** no party captures > 70% of total surplus; Gini coefficient of value distribution ≤ 0.5; each party ROI ≥ 5%.
- **Feasibility check:** resource availability, need/supply match, enhancer capability fit.
- **Risk check:** no party has > 3 active deals; no 3-strike cancelled/disputed pattern.
- **Compliance check:** parties verified, category not restricted, jurisdiction OK.

Validation produces a fairness score (0–100) and a list of violations/flags.

### 3.4 Value distribution models

- **Fixed Price:** consumer pays fixed amount, split among supplier/enhancer/platform.
- **Revenue Share:** output sold jointly, proceeds distributed by percentage.
- **Cost-Plus:** transparent cost stack + agreed margins.
- **Barter/Exchange:** non-monetary exchange, platform provides valuation guidance.
- **Hybrid:** combination of upfront, milestone, and revenue-share components.

### 3.5 Key supporting capabilities

- **Party & profile management** (individual, organization, party group).
- **Role profiles** per party: SupplierProfile, ConsumerProfile, EnhancerProfile.
- **Matching engine** (compatibility score across resource/need, geography, trust, timing, risk).
- **Multi-thread negotiation** on term dimensions with versioning.
- **Milestone tracking** with verification methods and conditional payment release.
- **Trust & reputation** score derived from reviews, history, verification, disputes.
- **Dispute resolution** (direct negotiation → platform mediation → binding arbitration).
- **Notifications** via email and in-app.

---

## 4. High-Level Architecture for the Deal Domain

We add a new **Deal bounded context** without disturbing existing user/identity code.

### 4.1 New domain aggregates

| Aggregate | Root | Purpose |
|---|---|---|
| Party | `Party` | Platform identity for any participant; one Party can have multiple role profiles (S/C/E) and can be an individual, organization, or group. |
| Deal | `Deal` | Central aggregate: lifecycle state, title/description, dates, location, totals, initiator. |
| DealParticipation | child of Deal | Links a Party to a Deal with a role and status. A single Party may participate in many Deals and may play a different role in each one; within any single Deal a Party may hold at most one role. |
| Resource | child of Deal | What the Supplier provides. |
| Need | child of Deal | What the Consumer wants. |
| Enhancement | child of Deal | What the Enhancer contributes. |
| Term | child of Deal | Negotiable clause with versioning. |
| ValueDistribution | child of Deal | Allocation of total value among parties and platform. |
| Milestone | child of Deal | Trackable deliverable with payment trigger. |
| Agreement | `Agreement` | Formal signed contract emerging from a locked deal. |
| TrustScore | `TrustScore` | Computed reputation per party. |
| Review | child of Deal | Post-deal rating/feedback. |
| Category | `Category` | Hierarchical taxonomy for domains, resource types, need types, enhancement types. |
| Match | `Match` | Platform-generated suggestion linking three parties. |
| Dispute | `Dispute` | Conflict record with resolution workflow. |

> **Important:** The existing `User` aggregate remains the authentication/identity root. A `Party` is a separate **business identity** that may be linked to one or more `User` accounts (for groups, organizations, or a single individual). A user can belong to multiple parties, selected at request time via an `X-Party-ID` header.
>
> **Party-to-Deal multiplicity:** A single `Party` may participate in many `Deal`s over time and may play a different `PartyRole` (Supplier, Consumer, Enhancer) in each one. Within any single `Deal`, however, a Party may hold at most one role, and the Deal must contain exactly one of each role.

### 4.2 New scopes

Extend the existing `role_definitions.scopes` model with deal-specific scopes:

- `parties:read`, `parties:write`
- `deals:read`, `deals:write`, `deals:transition`
- `terms:negotiate`
- `payments:read`, `payments:write`
- `admin:*`

Built-in platform roles can be expanded:
- `user` → add `parties:read`, `parties:write`, `deals:read`, `deals:write`, `deals:transition`, `terms:negotiate`
- `admin` → add `admin:*`

### 4.3 Cross-cutting extension points

- **Events:** introduce an `EventPublisher` port in `application` so that deal state changes can later be consumed by a separate matching, notification, or analytics service. The monolith can implement this in-memory or via Postgres `LISTEN/NOTIFY` initially, then replace with Kafka without touching domain/application code.
- **Payment provider:** define a `PaymentGateway`/`EscrowService` port. The MVP implements a "ledger-only" fake escrow (credits/debits in `platform_wallets`) without a real PSP.
- **Matching:** define a `MatchingEngine` port. The MVP uses a deterministic SQL+Rust scoring function.

---

## 5. Phased Implementation Roadmap

The PDF proposes a 24-month roadmap across four phases. The plan below compresses that into **milestones executable within the current monolith**, mapping each to the existing crates.

### Phase 0: Foundation (1–2 milestones)

**Goal:** Extend identity/authorization to support multiple business parties per user.

1. **Party aggregate & user-party membership**
   - Domain: `Party`, `PartyType`, `PartyRole`, `PartyMembership`, `UserPartyMembership`.
   - Application: `CreateParty`, `ListMyParties`, `GetParty`, `UpdateParty`, `AddPartyRole`, `UpdateRoleProfile`.
   - Infrastructure: Postgres repositories + migrations for `parties`, `party_roles`, `user_party_memberships`.
   - API: `POST /api/v1/parties`, `GET /api/v1/parties/me`, `GET|PUT|PATCH /api/v1/parties/{id}`, `POST|PUT|DELETE /api/v1/parties/{id}/roles/{role}`.
   - Auth: `X-Party-ID` header selection; reject deal endpoints without it.

2. **Category taxonomy**
   - Domain: `Category`, `CategoryType` (DOMAIN, RESOURCE_TYPE, NEED_TYPE, ENHANCEMENT_TYPE, LOCATION, CUSTOM).
   - Infrastructure: `categories` table with self-referencing parent.
   - Seed agriculture/real-estate/transportation/manufacturing/technology categories.

### Phase 1: Deal Lifecycle MVP (3–4 milestones)

**Goal:** A deal can be drafted, submitted, negotiated, locked, committed, executed, and completed.

3. **Deal & participation aggregate**
   - Domain: `Deal`, `DealStatus`, `DealParticipation`, `ParticipationStatus`, `DealRole`.
   - Invariants: exactly 3 participations per deal, one per role; initiator must be one of them.
   - Application: `CreateDraftDeal`, `GetDeal`, `ListDeals`, `UpdateDraftDeal`, `SubmitDeal`.
   - Infrastructure: `deals`, `deal_participations` tables.

4. **Resource, Need, Enhancement child aggregates**
   - Domain: `Resource`, `Need`, `Enhancement`.
   - Rules: at least one must be present based on initiator role; supplier-initiated deals require a resource, etc.
   - API: nested under `POST /api/v1/deals` and `GET /api/v1/deals/{id}`.

5. **State machine engine**
   - Domain: `DealStateMachine` with explicit allowed transitions and required preconditions.
   - Application: `ExecuteDealTransition` use case.
   - API: `POST /api/v1/deals/{id}/transitions`, `GET /api/v1/deals/{id}/transitions`.
   - Implement timeouts as a background worker (initially a simple Tokio cron task) that expires drafts/suggested/pending_review/negotiating/on-hold deals.

6. **Negotiation (single-thread MVP)**
   - Domain: `Term`, `TermType`, `TermStatus`, `TermVersion`.
   - Application: `ProposeTerm`, `CounterTerm`, `AcceptTerm`, `RejectTerm`, `ListTerms`.
   - API: `GET|POST /api/v1/deals/{id}/terms`, `POST /api/v1/deals/{id}/terms/{termId}/accept`, etc.

### Phase 2: Value Distribution & Validation (2 milestones)

**Goal:** Enforce the Win-Win-Win principle and support basic value models.

7. **Value distribution aggregate**
   - Domain: `ValueDistribution`, `DistributionModel`, `PaymentScheduleEntry`.
   - Invariants: percentages sum to 100, each party ≥ 5%, platform fee within configured bounds.
   - Application: `SetValueDistribution`, `GetValueDistribution`.

8. **Win-Win-Win validation engine**
   - Domain service: `WinWinWinValidator`.
   - Computes PVE, absolute gain, proportional fairness (Gini-inspired), market benchmark premium, opportunity cost.
   - Returns `ValidationResult` with score, violations, warnings, and party-specific feedback.
   - Integrated into `SubmitDeal`, `LockTerms`, and `Commit` transitions.
   - API: `POST /api/v1/deals/{id}/validate`.

### Phase 3: Agreement, Execution, & Settlement (2–3 milestones)

**Goal:** Convert a locked deal into a signed agreement with milestone-based escrow releases.

9. **Agreement & signatures**
   - Domain: `Agreement`, `Signature`, `SignatureType`.
   - MVP signature is a SHA-256 hash of agreement text + party ID + timestamp + an attestation string (not legally binding, but auditable).
   - Application: `GenerateAgreement`, `SignAgreement`.
   - API: `GET|POST /api/v1/deals/{id}/agreement`, `POST /api/v1/deals/{id}/agreement/sign`.

10. **Milestones & execution tracking**
    - Domain: `Milestone`, `MilestoneStatus`, `VerificationMethod`.
    - Application: `CreateMilestone`, `UpdateMilestone`, `VerifyMilestone`.
    - API: `GET|POST /api/v1/deals/{id}/milestones`, `PATCH /api/v1/deals/{id}/milestones/{mId}`, `POST .../verify`.

11. **Internal escrow/wallet (ledger-only)**
    - Domain: `PlatformWallet`, `Transaction`, `TransactionType`, `TransactionStatus`.
    - Ports: `EscrowService`, `PaymentGateway` (fake/stub implementation).
    - Consumer-funded escrow: consumer deposits to platform wallet, funds held, released on milestone verification.
    - API: `GET /api/v1/payments/wallets/me`, `POST /api/v1/payments/deposit`, `POST /api/v1/payments/withdraw`.

### Phase 4: Trust, Matching, Discovery (2 milestones)

**Goal:** Help parties find each other and build reputation.

12. **Matching engine v1**
    - Port: `MatchingEngine`.
    - Deterministic score across resource/need alignment, location, trust score, availability, value alignment.
    - API: `GET /api/v1/matches` (for current party), `POST /api/v1/matches/{id}/respond`.

13. **Reviews & trust score**
    - Domain: `Review`, `TrustScore`.
    - Application: `SubmitReview`, `GetTrustScore`, `RecalculateTrustScore`.
    - API: `POST /api/v1/deals/{id}/reviews`, `GET /api/v1/trust/party/{partyId}`.

### Phase 5: Advanced capabilities (future)

14. Multi-threaded negotiation with term dimension locking and ZOPA visualization.
15. Three-tier dispute resolution workflow.
16. Party groups with governance/voting.
17. External payment provider integration (Stripe/Flutterwave).
18. Event sourcing for deal lifecycle audit trail.
19. GraphQL/read-model projections (CQRS).
20. Extraction of contexts into microservices behind HTTP/gRPC adapters.

---

## 6. Detailed Crate-by-Crate Plan

### 6.1 `crates/domain`

#### 6.1.1 New modules

```
crates/domain/src/
├── entities/
│   ├── mod.rs              (re-exports)
│   ├── party.rs            (Party, PartyType, GeoPoint)
│   ├── party_role.rs       (SupplierProfile, ConsumerProfile, EnhancerProfile)
│   ├── category.rs         (Category, CategoryType)
│   ├── deal.rs             (Deal, DealStatus)
│   ├── deal_participation.rs
│   ├── resource.rs
│   ├── need.rs
│   ├── enhancement.rs
│   ├── term.rs             (Term, TermType, TermStatus)
│   ├── value_distribution.rs
│   ├── milestone.rs
│   ├── agreement.rs
│   ├── signature.rs
│   ├── review.rs
│   ├── trust_score.rs
│   ├── match_suggestion.rs
│   ├── dispute.rs
│   ├── wallet.rs
│   └── transaction.rs
├── repositories/
│   ├── mod.rs
│   ├── party_repository.rs
│   ├── category_repository.rs
│   ├── deal_repository.rs
│   ├── term_repository.rs
│   ├── value_distribution_repository.rs
│   ├── milestone_repository.rs
│   ├── agreement_repository.rs
│   ├── review_repository.rs
│   ├── trust_score_repository.rs
│   ├── match_repository.rs
│   ├── dispute_repository.rs
│   └── wallet_repository.rs
├── services/
│   ├── mod.rs
│   ├── deal_state_machine.rs
│   └── win_win_win_validator.rs
└── errors.rs               (extended DomainError variants)
```

#### 6.1.2 Value objects to add

- `Money { amount: Decimal, currency: String }`
- `GeoPoint { latitude, longitude }`
- `Percentage { value: Decimal }` clamped 0–100
- `DealReference` (human-readable `DL-YYYY-NNNN`)
- `PartyRole` enum: `Supplier`, `Consumer`, `Enhancer`
- `DealStatus` enum: 13 states
- `VerificationStatus` enum: `Unverified`, `Pending`, `Verified`, `Rejected`

#### 6.1.3 Domain invariants (selected critical ones)

- A `Deal` must have exactly three `DealParticipation` records, one per `PartyRole`.
- The initiator party must be among participations with `is_initiator = true` and its role must match `initiator_role`.
- A single `Party` may participate in many `Deal`s and may play a different `PartyRole` in each `Deal`; a Party may hold at most one role within a single Deal.
- `ValueDistribution` percentages must sum to 100 within epsilon.
- `Deal` state transitions are only permitted via the state machine.
- A `Term` cannot be modified after `TermStatus::Accepted` (immutability + versioning).
- A `Signature` references an existing `Agreement` and authorized party member.

#### 6.1.4 Domain error extensions

Add to `DomainError`:

- `InvalidPartyName`, `InvalidDealTitle`, `InvalidMoneyAmount`
- `PartyNotFound`, `DealNotFound`, `TermNotFound`
- `InvalidStateTransition { from, to }`
- `WinWinWinValidationFailed { violations }`
- `DuplicatePartyRole`
- `InsufficientPermissions`
- `RepositoryError(String)` (already exists)

### 6.2 `crates/application`

#### 6.2.1 New modules

```
crates/application/src/
├── parties/
│   ├── create_party.rs
│   ├── get_party.rs
│   ├── list_my_parties.rs
│   ├── update_party.rs
│   ├── add_party_role.rs
│   └── update_role_profile.rs
├── deals/
│   ├── create_deal.rs
│   ├── get_deal.rs
│   ├── list_deals.rs
│   ├── update_deal.rs
│   ├── submit_deal.rs
│   ├── execute_transition.rs
│   └── list_transitions.rs
├── terms/
│   ├── propose_term.rs
│   ├── accept_term.rs
│   ├── reject_term.rs
│   └── list_terms.rs
├── value_distribution/
│   ├── set_value_distribution.rs
│   └── validate_deal.rs
├── agreements/
│   ├── generate_agreement.rs
│   └── sign_agreement.rs
├── milestones/
│   ├── create_milestone.rs
│   ├── update_milestone.rs
│   └── verify_milestone.rs
├── matching/
│   └── find_matches.rs
├── reviews/
│   └── submit_review.rs
├── trust/
│   └── get_trust_score.rs
└── events/
    ├── mod.rs
    └── event_publisher.rs    (new outbound port)
```

#### 6.2.2 New outbound ports

- `EventPublisher: Send + Sync` with `async fn publish(&self, event: DomainEvent) -> Result<(), ApplicationError>`.
- `PaymentGateway: Send + Sync` with deposit/withdraw/hold/release primitives.
- `MatchingEngine: Send + Sync` with `async fn find_matches(&self, party_id, role) -> Result<Vec<MatchSuggestion>, ApplicationError>`.
- `DocumentRenderer: Send + Sync` for generating agreement text from locked terms (initially a template engine; can be replaced with a PDF service).

#### 6.2.3 DTO patterns

Follow existing pattern: each use case defines its own command/result structs (e.g., `CreateDealCommand`, `CreateDealResult`). Commands contain raw strings/numbers and are validated before constructing domain value objects. Results are flat JSON-friendly structs.

#### 6.2.4 Application error extensions

Add to `ApplicationError`:

- `DealNotFound`, `PartyNotFound`, `TermNotFound`
- `InvalidStateTransition`
- `WinWinWinValidationFailed { violations }`
- `DealAccessDenied`
- `PaymentFailed`
- `MatchingEngineUnavailable`

### 6.3 `crates/infrastructure`

#### 6.3.1 New modules

```
crates/infrastructure/src/
├── repositories/
│   ├── postgres_party_repository.rs
│   ├── postgres_category_repository.rs
│   ├── postgres_deal_repository.rs
│   ├── postgres_term_repository.rs
│   ├── postgres_value_distribution_repository.rs
│   ├── postgres_milestone_repository.rs
│   ├── postgres_agreement_repository.rs
│   ├── postgres_review_repository.rs
│   ├── postgres_trust_score_repository.rs
│   ├── postgres_match_repository.rs
│   ├── postgres_dispute_repository.rs
│   └── postgres_wallet_repository.rs
├── matching/
│   └── sql_matching_engine.rs
├── payments/
│   └── ledger_escrow_service.rs
└── events/
    └── in_memory_event_publisher.rs
```

#### 6.3.2 Repository implementation notes

- Use `sqlx::query!` macros and add new `.sqlx/` offline metadata via `cargo sqlx prepare --workspace`.
- Map unique constraints to domain errors:
  - `parties_email_key` → `DomainError::DuplicateEmail` (or a new `DuplicatePartyEmail`).
  - `party_roles_party_id_role_type_key` → `DomainError::DuplicatePartyRole`.
- Implement aggregate-scoped fetching (e.g., `DealRepository::find_by_id` joins `deal_participations`, `resource`, `need`, `enhancement`, `value_distribution`, `terms`, `milestones` as needed, or expose eager-load flags).
- Use Postgres `FOR UPDATE` only inside explicit transition use cases to avoid lock contention.

#### 6.3.3 Matching engine (v1)

`SqlMatchingEngine` runs a Rust scoring function after fetching candidate parties from Postgres:

1. Filter by role, active status, verification, service radius, availability window.
2. For each candidate triplet, compute a weighted score across:
   - Resource/need category alignment
   - Geographic proximity
   - Temporal overlap
   - Trust score
   - Value alignment
   - Historical success rate
   - Risk profile
3. Return top N suggestions.

#### 6.3.4 Escrow/payments (v1)

`LedgerEscrowService` maintains `platform_wallets` and `transactions` tables. No real PSP. Operations:

- `deposit(party_id, amount)` — credits wallet.
- `hold(deal_id, party_id, amount)` — moves wallet balance to `escrow_balance`.
- `release(deal_id, from_party, to_party, amount)` — moves from escrow to recipient wallet.
- `refund(deal_id, party_id, amount)` — returns escrow to originating wallet.

A stub `PaymentGateway` records external reference IDs for future integration.

#### 6.3.5 Event publisher (v1)

`InMemoryEventPublisher` publishes to a Tokio broadcast channel. A background consumer logs events and can later be replaced with Kafka/Redis Streams.

### 6.4 `crates/api`

#### 6.4.1 New routes

```rust
// parties.rs
POST   /api/v1/parties
GET    /api/v1/parties/me
GET    /api/v1/parties/{id}
PUT    /api/v1/parties/{id}
PATCH  /api/v1/parties/{id}
POST   /api/v1/parties/{id}/roles
PUT    /api/v1/parties/{id}/roles/{role}
DELETE /api/v1/parties/{id}/roles/{role}

// deals.rs
POST   /api/v1/deals
GET    /api/v1/deals
GET    /api/v1/deals/{id}
PUT    /api/v1/deals/{id}
PATCH  /api/v1/deals/{id}
GET    /api/v1/deals/{id}/history
POST   /api/v1/deals/{id}/transitions
GET    /api/v1/deals/{id}/transitions

// terms.rs
GET    /api/v1/deals/{id}/terms
POST   /api/v1/deals/{id}/terms
POST   /api/v1/deals/{id}/terms/{termId}/accept
POST   /api/v1/deals/{id}/terms/{termId}/reject
POST   /api/v1/deals/{id}/terms/{termId}/counter

// value_distribution.rs
GET    /api/v1/deals/{id}/value-distribution
PUT    /api/v1/deals/{id}/value-distribution
POST   /api/v1/deals/{id}/validate

// milestones.rs
GET    /api/v1/deals/{id}/milestones
POST   /api/v1/deals/{id}/milestones
PATCH  /api/v1/deals/{id}/milestones/{mId}
POST   /api/v1/deals/{id}/milestones/{mId}/verify

// agreements.rs
GET    /api/v1/deals/{id}/agreement
POST   /api/v1/deals/{id}/agreement
POST   /api/v1/deals/{id}/agreement/sign

// matching.rs
GET    /api/v1/matches
POST   /api/v1/matches/{id}/respond

// reviews.rs
POST   /api/v1/deals/{id}/reviews

// trust.rs
GET    /api/v1/trust/party/{partyId}

// payments.rs
GET    /api/v1/payments/wallets/me
POST   /api/v1/payments/deposit
POST   /api/v1/payments/withdraw
```

#### 6.4.2 Auth/authorization changes

- Extend `AuthContext` (in middleware) to optionally include `party_id` from `X-Party-ID`.
- Add helpers:
  - `require_party_scope(ctx, scope)`
  - `require_deal_participant(ctx, deal_id, repo)`
  - `require_deal_role(ctx, deal_id, role, repo)`
- Public routes remain: health, user registration/login, email verification, password reset.
- Deal routes require authentication and valid `X-Party-ID`.

#### 6.4.3 DTOs

Create `crates/api/src/dto/deals.rs`, `dto/parties.rs`, etc., mirroring the request/response shapes in the PDF, trimmed to the MVP subset.

#### 6.4.4 `AppState` extensions

Add new use-case instances to `AppState` and `main.rs` wiring.

---

## 7. Database Schema Additions

All additions are new tables; **no breaking changes to existing `users`, `role_definitions`, `email_verifications`, `password_resets` tables**.

### 7.1 Core identity/party tables

```sql
CREATE TABLE parties (
    id UUID PRIMARY KEY,
    party_type TEXT NOT NULL CHECK (party_type IN ('INDIVIDUAL','ORGANIZATION','PARTY_GROUP')),
    display_name CITEXT NOT NULL,
    email CITEXT NOT NULL UNIQUE,
    phone TEXT,
    tax_id TEXT,
    verification_status TEXT NOT NULL DEFAULT 'UNVERIFIED',
    primary_domain_id UUID REFERENCES categories(id),
    location_geo GEOGRAPHY(POINT),
    location_address JSONB,
    service_radius_km DECIMAL,
    trust_score DECIMAL NOT NULL DEFAULT 0,
    total_deals_completed INTEGER NOT NULL DEFAULT 0,
    total_deals_initiated INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE user_party_memberships (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    party_id UUID NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
    member_role TEXT NOT NULL DEFAULT 'MEMBER', -- OWNER, ADMIN, MEMBER
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, party_id)
);

CREATE TABLE party_roles (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
    role_type TEXT NOT NULL CHECK (role_type IN ('SUPPLIER','CONSUMER','ENHANCER')),
    profile JSONB NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT true,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (party_id, role_type)
);
```

### 7.2 Category table

```sql
CREATE TABLE categories (
    id UUID PRIMARY KEY,
    parent_category_id UUID REFERENCES categories(id),
    category_name CITEXT NOT NULL,
    category_code CITEXT NOT NULL UNIQUE,
    description TEXT,
    category_type TEXT NOT NULL CHECK (category_type IN ('DOMAIN','RESOURCE_TYPE','NEED_TYPE','ENHANCEMENT_TYPE','LOCATION','CUSTOM')),
    icon_url TEXT,
    metadata_schema JSONB,
    is_active BOOLEAN NOT NULL DEFAULT true,
    display_order INTEGER NOT NULL DEFAULT 1,
    deal_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 7.3 Deal tables

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
    location_geo GEOGRAPHY(POINT),
    location_address JSONB,
    total_deal_value DECIMAL,
    currency TEXT,
    platform_fee_percentage DECIMAL,
    platform_fee_amount DECIMAL,
    win_win_win_validated BOOLEAN NOT NULL DEFAULT false,
    validation_checked_at TIMESTAMPTZ,
    validation_score DECIMAL,
    validation_result JSONB,
    is_public BOOLEAN NOT NULL DEFAULT true,
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
    -- Each deal has exactly one party per role.
    UNIQUE (deal_id, role),
    -- A party may participate in many deals, but may hold at most one role within a single deal.
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

### 7.4 Child deal tables

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
    currency TEXT NOT NULL,
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

### 7.5 Agreement, trust, matching, payments

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
    balance DECIMAL NOT NULL DEFAULT 0,
    escrow_balance DECIMAL NOT NULL DEFAULT 0,
    pending_balance DECIMAL NOT NULL DEFAULT 0,
    total_deposited DECIMAL NOT NULL DEFAULT 0,
    total_withdrawn DECIMAL NOT NULL DEFAULT 0,
    currency TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE transactions (
    id UUID PRIMARY KEY,
    deal_id UUID REFERENCES deals(id),
    agreement_id UUID REFERENCES agreements(id),
    transaction_type TEXT NOT NULL,
    from_party_id UUID REFERENCES parties(id),
    to_party_id UUID REFERENCES parties(id),
    amount DECIMAL NOT NULL,
    currency TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'PENDING',
    payment_method TEXT,
    external_reference TEXT,
    executed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 7.6 Indexes

Add indexes for performance-critical lookups:

```sql
CREATE INDEX idx_deals_status ON deals(deal_status);
CREATE INDEX idx_deals_initiator ON deals(initiator_party_id);
CREATE INDEX idx_deals_domain ON deals(domain_category_id);
CREATE INDEX idx_deals_geo ON deals USING GIST(location_geo);
CREATE INDEX idx_participations_party ON deal_participations(party_id);
CREATE INDEX idx_participations_deal ON deal_participations(deal_id);
CREATE INDEX idx_parties_geo ON parties USING GIST(location_geo);
CREATE INDEX idx_match_suggestions_supplier ON match_suggestions(supplier_party_id);
CREATE INDEX idx_match_suggestions_consumer ON match_suggestions(consumer_party_id);
CREATE INDEX idx_match_suggestions_enhancer ON match_suggestions(enhancer_party_id);
CREATE INDEX idx_deal_history_deal ON deal_history(deal_id, created_at DESC);
```

---

## 8. State Machine Implementation Plan

### 8.1 Rust representation

A domain service `DealStateMachine` with:

```rust
pub fn allowed_transitions(status: DealStatus) -> Vec<DealTransition>;
pub fn can_transition(deal: &Deal, transition: DealTransition, actor: &AuthContext) -> Result<(), DomainError>;
pub fn apply(deal: &mut Deal, transition: DealTransition, reason: Option<String>);
```

Transition preconditions are checked in the application use case before mutating the aggregate.

### 8.2 Timeout handling

A background task spawned in `main.rs` (reusing the Tokio runtime) queries:

```sql
SELECT id, deal_status, current_state_entered_at FROM deals
WHERE deal_status IN ('DRAFT','SUGGESTED','PENDING_REVIEW','NEGOTIATING','TERMS_LOCKED','ON_HOLD','AWAITING_PARTY')
  AND current_state_entered_at < now() - interval '...'
```

and executes transitions (`EXPIRE`, `AUTO_DELETE_DRAFT`, `AUTO_CANCEL_TERMS_LOCKED`, `MOVE_TO_ON_HOLD`) via the `ExecuteDealTransition` use case.

### 8.3 Validation gates by transition

| Transition | Validation gate |
|---|---|
| `DRAFT → SUGGESTED` | All participations present; basic field validation; at least resource/need/enhancement per initiator role. |
| `SUGGESTED → PENDING_REVIEW` | All invited parties exist and are active/verified. |
| `PENDING_REVIEW → NEGOTIATING` | All parties acknowledged. |
| `NEGOTIATING → TERMS_LOCKED` | All terms accepted or mandatory terms resolved. |
| `TERMS_LOCKED → COMMITTED` | Value distribution present; Win-Win-Win validation passes (critical rules); agreement generated; all signatures collected; escrow funded. |
| `COMMITTED → EXECUTING` | 3-day prep elapsed or unanimous begin-early. |
| `EXECUTING → COMPLETED` | All milestones verified; payments released; reviews optionally submitted. |

---

## 9. Win-Win-Win Validation Engine Plan

### 9.1 Location

`crates/domain/src/services/win_win_win_validator.rs` as a pure function/service with no dependencies on repositories.

### 9.2 Inputs

A `DealValidationInput` containing:

- `ValueDistribution`
- `Resource`, `Need`, `Enhancement`
- `Party` snapshots (trust score, verification level, active deals count)
- Domain config (min deal size, discount rate, min/max share thresholds)
- Optional market benchmarks (from a `MarketBenchmarkProvider` port for application layer)

### 9.3 Algorithm steps

1. **Normalize to PVE** using domain discount rate for revenue share/deferred obligations.
2. **Critical rules** (BLOCK if violated):
   - All party gains > 0.
   - No share > 70%.
   - Consumer cost < independent sourcing cost × 0.95 (or ≤ 1.05 with additional value).
   - Deal value ≥ min deal size.
3. **Warning rules** (FLAG + require acknowledgment):
   - Any share < 10%.
   - Enhancer fee/value-added outside 50–150%.
   - Risk ratio between max/min party > 3.0.
4. **Fairness score** (0–100) weighted:
   - Absolute gain: 25%
   - Proportional fairness (variance penalty): 30%
   - Market benchmark: 25%
   - Opportunity cost: 20%
5. Return `ValidationResult` with score, status (`Excellent`/`Good`/`Fair`/`Poor`/`Blocked`), violations, warnings, and per-party feedback.

### 9.4 Integration

- `CreateDraftDeal` and `SetValueDistribution` run validation in advisory mode (returns feedback but does not block).
- `SubmitDeal` and `LockTerms` require at least `Good` score and no critical violations.
- `Commit` re-runs validation as a hard gate.

---

## 10. Testing Strategy

Follow and extend the existing test patterns:

### 10.1 Domain unit tests

- Invariant tests for `Deal`, `ValueDistribution`, `Term`.
- State machine transition matrix tests.
- Win-Win-Win validator tests with sample deals (good, unbalanced, blocked).

### 10.2 Application unit tests

- Fake repositories for each new aggregate in `crates/application/src/test_helpers.rs`.
- Use-case tests for `CreateParty`, `CreateDeal`, `SubmitDeal`, `ExecuteTransition`, `SetValueDistribution`, `SignAgreement`, `VerifyMilestone`.
- Edge cases: duplicate role, missing participation, invalid transition, insufficient escrow.

### 10.3 Infrastructure integration tests

- Postgres repository tests in `crates/infrastructure/tests/`.
- Migration tests using `sqlx::test` against a local PostgreSQL instance.

### 10.4 API integration tests

- Extend `crates/api/tests/` with happy-path and error-path scenarios for deal creation, transitions, and validation.
- Use the existing in-memory fakes where possible; for deal routes that depend on parties, use a seeded database.

### 10.5 CI updates

- `cargo fmt --check && cargo clippy -- -D warnings`
- `cargo test`
- `cargo sqlx prepare --workspace` must remain up to date after each migration.

---

## 11. Migration Ordering

Create migrations as separate, idempotent, backwards-compatible files:

1. `create_categories_table.sql`
2. `create_parties_table.sql`
3. `create_user_party_memberships_table.sql`
4. `create_party_roles_table.sql`
5. `create_deals_table.sql`
6. `create_deal_participations_table.sql`
7. `create_resources_table.sql`
8. `create_needs_table.sql`
9. `create_enhancements_table.sql`
10. `create_terms_table.sql`
11. `create_value_distributions_table.sql`
12. `create_milestones_table.sql`
13. `create_agreements_table.sql`
14. `create_signatures_table.sql`
15. `create_reviews_table.sql`
16. `create_trust_scores_table.sql`
17. `create_match_suggestions_table.sql`
18. `create_platform_wallets_table.sql`
19. `create_transactions_table.sql`
20. `create_deal_history_table.sql`
21. `add_deal_indexes.sql`
22. `seed_categories_and_update_role_scopes.sql`

---

## 12. Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Schema becomes very large; offline sqlx metadata hard to maintain. | Medium | Add tables incrementally; run `cargo sqlx prepare` after each milestone; keep `.sqlx/` in version control. |
| Win-Win-Win algorithm depends on market benchmarks not yet available. | High | Start with configurable defaults and party-provided opportunity-cost estimates; build a `MarketBenchmarkProvider` port that can be stubbed. |
| Real payment/escrow integration is complex and regulated. | High | Use internal ledger only in MVP; isolate real PSP behind `PaymentGateway` port. |
| Multi-party auth (`X-Party-ID`) adds request complexity. | Medium | Default `X-Party-ID` to the user's only active party when absent; require explicit header only when user has multiple parties. |
| Existing `Role`/`role_definitions` naming collides with deal roles. | Low | Rename current platform roles concept where ambiguous (`PlatformRole` vs `DealRole`); keep database names unchanged to avoid migration churn. |
| Monolith may become too large before extraction. | Medium | Enforce strict module boundaries per bounded context; use internal event publisher; document service boundaries for future extraction. |

---

## 13. Success Criteria for MVP

- A user can register, verify email, create a party, and assign Supplier/Consumer/Enhancer role profiles.
- Any role can initiate a deal with the other two roles invited.
- Deal proceeds through DRAFT → SUGGESTED → PENDING_REVIEW → NEGOTIATING → TERMS_LOCKED → COMMITTED → EXECUTING → COMPLETED.
- Parties can propose/accept/reject terms and set a value distribution.
- Win-Win-Win validation blocks deals with critical violations and flags warnings.
- A template agreement is generated and digitally signed by all parties.
- Milestones can be tracked and verified; internal escrow funds are released on completion.
- Post-deal reviews update trust scores.
- Basic matching returns ranked suggestions for missing roles.
- All existing user/auth tests continue to pass.

---

## 14. Next Immediate Steps

1. **Review and approve this plan.**
2. **Create a feature branch** and implement Phase 0 (Party aggregate, user-party membership, category taxonomy).
3. **Update role definitions** with new scopes and seed initial categories.
4. **Add domain/application tests** for Phase 0 before writing infrastructure/API code.
5. **Proceed incrementally** through Phase 1 milestones, keeping migrations and `.sqlx/` metadata current.

---

## 15. Glossary (MVP subset)

| Term | Meaning |
|---|---|
| Party | A business participant (individual, organization, or group) on the platform. Distinct from `User`, which is an authentication account. |
| Deal | A 3-party arrangement with a Supplier, Consumer, and Enhancer around a value exchange. |
| DealRole | One of `Supplier`, `Consumer`, `Enhancer` within a specific deal. |
| Participation | The link between a Party and a Deal in a specific role. |
| Term | A negotiable clause (price, timeline, deliverables, etc.) with versioning. |
| ValueDistribution | Allocation of total deal value among parties and platform, including payment schedule. |
| Win-Win-Win Validation | Domain service ensuring all three parties achieve positive net value with fair distribution. |
| Match | Platform-generated compatibility suggestion linking three parties. |
| Escrow | Internal ledger holding consumer funds until milestones are verified. |
| TrustScore | Composite reputation metric per party derived from reviews and history. |
