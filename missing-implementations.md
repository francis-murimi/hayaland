# Hayaland — Missing Implementations & Feature Roadmap

> Generated after a full review of the codebase, design documents, migrations, and source tree.
> Goal: identify what is missing or improvable to make the 3-party deal platform comprehensive.

---

## 1. Current State Summary

Hayaland is a **solid, production-quality Rust monolith** built with Actix Web, PostgreSQL/sqlx, and hexagonal/clean architecture. The following capabilities are already implemented and tested:

| Domain | Status |
|--------|--------|
| Identity & Access | JWT auth, Argon2 hashing, email verification, password reset, RBAC scopes |
| Parties & Profiles | CRUD, role profiles (Supplier/Consumer/Enhancer), memberships, PostGIS geo search, nearby discovery |
| Deal Lifecycle | 13-state state machine, create/submit/transition, strict privacy controls |
| Negotiation | Terms (propose/counter/accept/reject/withdraw), value distribution |
| Win-Win-Win | Validator with fairness score, critical/warning rules, per-party feedback |
| Agreements | Auto-generated from locked terms, digital attestation signing, versioning |
| Milestones | Full lifecycle (start/complete/verify), escrow release on verification |
| Payments | Points ledger, wallets, escrow, multi-party transaction approval workflow |
| Reviews & Disputes | Multi-dimensional reviews, dispute evidence/resolution/severity/outcome |
| Verifications | Admin-approved verification levels that feed trust score |
| Trust Score | 8-component calculator, recalculation port, nightly worker, public cache sync |
| Messaging | AES-256-GCM encrypted messages, chat rooms, WebSocket, reactions, read receipts, admin broadcast |
| Background Workers | Email worker, deal timeout worker, trust-score worker |

The largest remaining gaps are **marketplace/discovery**, **notifications**, **event-driven infrastructure**, **search/analytics**, **media/document handling**, and **advanced execution models**.

---

## 2. Highest Priority — Missing MVP Features

These gaps prevent Hayaland from being a complete 3-party marketplace and should be addressed first.

### 2.1 Notification Service

**Status:** Completely absent — no tables, entities, ports, routes, or workers.

**What to implement:**
- `notifications` table:
  - `user_id`, `party_id`, `type`, `title`, `body`, `channel`, `status`, `read_at`, `metadata`, `topic`
- Notification preferences per user/party:
  - Channels: email, in-app, SMS, push
  - Quiet hours, per-topic subscriptions
- Notification templates (per event type, per locale)
- In-app notification center API:
  - `GET /api/v1/notifications`
  - `POST /api/v1/notifications/{id}/mark-read`
  - `DELETE /api/v1/notifications/{id}`
- Topic subscriptions: `deal:{id}`, `party:{id}:matches`, `system`
- Background delivery worker with retry tracking
- Integration points at key lifecycle events:
  - Deal invited / submitted / terms locked / signed / committed / completed / disputed
  - Milestone started / completed / verified
  - Transaction pending approval / approved / rejected
  - New message / review / verification result

**Why it matters:** Today, clients must poll. A notification layer is essential for user engagement and trust.

---

### 2.2 Marketplace Catalog: Resources, Needs, Enhancements

**Status:** Database tables exist (`resources`, `needs`, `enhancements`), but there are **no domain entities, repositories, use cases, or routes**.

**What to implement:**
- Domain entities `Resource`, `Need`, `Enhancement` with validation
- Repositories and CRUD use cases following the existing hexagonal pattern
- Link catalog items to deals during creation or via discovery
- REST APIs:
  - `POST /api/v1/resources`, `GET /api/v1/resources`, `GET /api/v1/resources/{id}`
  - `POST /api/v1/needs`, `GET /api/v1/needs`
  - `POST /api/v1/enhancements`, `GET /api/v1/enhancements`
  - `GET /api/v1/parties/{id}/resources`
- Full-text and faceted search by category, location, availability, quantity
- Support public listings so suppliers can advertise idle assets

**Why it matters:** Without a catalog, the platform is only a deal-closing tool, not a discovery marketplace.

---

### 2.3 Matching & Discovery Engine

**Status:** The `match_suggestions` table exists, but no code references it.

**What to implement:**
- `MatchSuggestion` domain entity and `MatchingEngine` application port
- Deterministic compatibility scoring across 7 dimensions:
  - Resource/need alignment
  - Geographic fit
  - Temporal availability
  - Trust score
  - Value alignment
  - Historical success
  - Risk profile
- REST APIs:
  - `GET /api/v1/matches` — ranked suggestions for the current party
  - `POST /api/v1/matches/{id}/respond` — accept / decline / counter-propose
  - `GET /api/v1/discovery/domains` — browse by category
  - `GET /api/v1/discovery/deals` — public deal opportunities
- Convert an accepted match into a `Deal` draft with pre-filled participations

**Why it matters:** Algorithmic triadic matching is the core differentiator of the 3-party model.

---

### 2.4 Admin Audit Log & Platform Configuration

**Status:** Admin actions are scattered across modules; no centralized audit log or runtime config store.

**What to implement:**
- `admin_actions` table:
  - `admin_user_id`, `action_type`, `target_type`, `target_id`, `before_snapshot`, `after_snapshot`, `reason`, `ip_address`, `created_at`
- `platform_config` table for runtime settings:
  - Min deal size, fee bounds, validation thresholds, governing law defaults
- REST APIs:
  - `GET /api/v1/admin/audit-log`
  - `GET /api/v1/admin/config`
  - `PATCH /api/v1/admin/config`
- Emit audit events from all admin endpoints (agreement edits, dispute resolutions, verification approvals, forced transitions)

**Why it matters:** Required for regulatory compliance, dispute evidence, and operational safety.

---

## 3. Deal Execution Hardening

These features make the deal lifecycle robust enough for real-world usage.

### 3.1 Automatic Escrow Funding on Commit

**Status:** The `TERMS_LOCKED → COMMITTED` transition exists, but no automatic escrow hold is enforced.

**What to implement:**
- On transition to `COMMITTED`, automatically create an `ESCROW_HOLD` transaction for the consumer's obligation
- Block `COMMITTED → EXECUTING` until escrow is funded or funding is in progress
- Configurable grace period per deal
- Notify consumer and other parties of funding status

---

### 3.2 Settlement Saga

**Status:** Milestones trigger individual escrow releases, but there is no end-to-end settlement orchestration.

**What to implement:**
- `SettlementSaga` use case invoked on deal completion
- Steps:
  1. Finalize all milestone releases
  2. Deduct platform fee
  3. Release remaining escrow to supplier/enhancer per value distribution
  4. Update trust scores
  5. Mark deal `COMPLETED`
  6. Request reviews from all parties
- Compensation steps for rollback on failure
- `settlement_sagas` state table for observability

---

### 3.3 Refunds, Chargebacks & Manual Adjustments

**Status:** `RecordAdjustment` exists, but no structured refund workflow.

**What to implement:**
- `RefundRequest` entity with reason, amount, and approval workflow
- REST APIs:
  - `POST /api/v1/deals/{id}/refunds`
  - `GET /api/v1/admin/refunds`
  - `POST /api/v1/admin/refunds/{id}/approve`
- Link refunds to cancellation or dispute resolution
- Chargeback limits and fraud signals

---

### 3.4 Multi-Model Value Distribution

**Status:** Only the fixed-price path is realistically handled.

**What to implement:**
- Full support for `RevenueShare`, `CostPlus`, `Barter`, and `Hybrid` models in the validator and settlement logic
- `MarketBenchmarkProvider` port (stub initially, pluggable to real data)
- Dynamic pricing suggestions endpoint
- ZOPA visualization data for negotiation UI

---

## 4. Platform Intelligence & UX

### 4.1 Search Service

**What to implement:**
- Elasticsearch or PostgreSQL `tsvector` index for:
  - Parties (display name, description, roles, domain)
  - Public deals
  - Resources / Needs / Enhancements
- Faceted search by category, location, verification status, trust tier
- `GET /api/v1/search?type=party&q=...&filters=...`

---

### 4.2 Analytics & Reporting

**What to implement:**
- Read-model tables or materialized views:
  - Daily deal volume, completion rate, dispute rate
  - Top categories, average deal value
  - Party activity metrics
- Admin dashboard endpoints:
  - `GET /api/v1/admin/metrics/dashboard`
  - `GET /api/v1/admin/reports/deals`
  - `GET /api/v1/admin/reports/trust`

---

### 4.3 Media & Document Service

**Status:** Messages can store attachment URLs, but there is no upload infrastructure.

**What to implement:**
- Object-storage upload endpoint with presigned URLs
- Virus scanning port (ClamAV stub)
- OCR port for document verification
- Agreement PDF generation and storage
- Evidence uploads for disputes and verifications

---

## 5. Cross-Cutting Platform Hardening

### 5.1 Event-Driven Architecture

**Status:** Only an in-memory realtime publisher exists for WebSocket messages.

**What to implement:**
- Domain event envelope schema:
  - `event_id`, `aggregate_id`, `correlation_id`, `causation_id`, `occurred_at`
- Event bus port with Kafka / Redis Streams / RabbitMQ implementations
- Core events:
  - `DealCreated`, `DealStateChanged`, `AgreementSigned`, `EscrowReleased`, `TrustScoreUpdated`, `NotificationSent`
- Projection workers for search index, analytics, notifications

---

### 5.2 Idempotency Keys & Rate Limiting

**What to implement:**
- `idempotency_keys` table
- Middleware to enforce idempotency on mutation endpoints
- Per-user and per-IP rate limiting (Redis-backed)
- Expose `Retry-After` headers

---

### 5.3 OAuth2 / OIDC Authentication

**Status:** Only email/password JWT authentication exists.

**What to implement:**
- OAuth2 port with Google, Apple, Microsoft providers
- Account linking (OAuth identity → existing user)
- MFA support (TOTP / SMS)

---

### 5.4 ABAC & Fine-Grained Authorization

**Status:** Only RBAC scopes exist.

**What to implement:**
- Attribute-based checks: party membership role, deal role, verification level, deal status
- Policy engine for who can sign, approve transactions, or initiate high-value deals

---

## 6. Advanced Domain Features

### 6.1 Party Groups & Governance

**Status:** The `PARTY_GROUP` enum variant exists, but no group logic is implemented.

**What to implement:**
- Tables: `party_groups`, `party_group_memberships`, `party_group_actions`, `party_group_votes`
- Governance models: `ANY_ONE`, `MAJORITY`, `ALL`, `THRESHOLD`, `WEIGHTED`
- Voting workflow for deal actions and signatures
- Contribution percentages feeding value distribution

---

### 6.2 Public Deal Discovery & Deal Templates

**What to implement:**
- Make `is_public = true` deals browsable via discovery API
- Deal templates per category with pre-filled terms and value distributions
- Clone deal endpoint
- Invitation workflow by email/link for parties not yet on the platform

---

### 6.3 E-Signature Integration

**Status:** Only SHA-256 attestation signatures are supported.

**What to implement:**
- DocuSign / HelloSign adapter port
- Qualified eIDAS signature support
- Webhook handlers for signature completion
- Rendered PDF agreement storage

---

### 6.4 Smart Contract / Oracle Execution Layer

**What to implement:**
- Oracle port for weather, delivery, IoT, and price feeds
- Conditional milestone triggers (time-based, oracle-based, third-party verify)
- Blockchain anchoring of agreement hashes
- Smart-contract escrow for crypto settlement

---

## 7. Recommended Implementation Order

| Phase | Focus | Key Deliverables |
|-------|-------|------------------|
| **1** | Complete MVP marketplace | Notification service, Resource/Need/Enhancement catalog, matching engine, admin audit log |
| **2** | Harden deal execution | Auto escrow funding, settlement saga, refunds, multi-model value distribution |
| **3** | Platform intelligence | Search service, analytics dashboard, media uploads, PDF generation |
| **4** | Scale & resilience | Event bus, idempotency, rate limiting, OAuth2, ABAC, Redis caching |
| **5** | Advanced domain | Party groups, public discovery, e-signatures, oracles, smart contracts |

---

## 8. Quick Wins to Start Immediately

1. **Notification service** — high impact, clearly scoped, builds on existing messaging/transaction events.
2. **Resource / Need / Enhancement CRUD** — tables already exist; mostly wiring work.
3. **Matching engine v1** — deterministic SQL+Rust scoring against existing `parties` and `categories`.
4. **Admin audit log** — additive table, low risk, high compliance value.
5. **Automatic escrow funding** — closes the deal lifecycle gap.

These five features would transform Hayaland from a well-built deal-management API into a functioning 3-party marketplace.
