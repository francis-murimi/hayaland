# Gap Analysis: Current Hayaland vs. `3partydeal.pdf`

> **Scope:** High-level inventory of what is already implemented and what still needs to be added so the codebase fully covers the `3partydeal.pdf` Software Design Document.
> **Status snapshot:** After the trust-score module, the core deal lifecycle is largely in place. The biggest remaining gaps are marketplace/discovery, notifications, event-driven infrastructure, search/analytics, media, and the surrounding cross-cutting platform services.

## 1. What is already implemented (MVP foundation)

| Area | Implemented |
|------|-------------|
| **Identity & Access** | User CRUD, JWT login, email verification, password reset, role-based scopes (`admin:*`, `deals:write`, etc.) |
| **Parties** | Party CRUD, party roles (Supplier/Consumer/Enhancer) with role profiles, geo search/nearby, trust-score read/recalc endpoints |
| **Deal lifecycle** | 13-state state machine, create/update/submit/transition, terms (propose/counter/accept/reject/withdraw), value distribution, Win-Win-Win validator, agreement generation & signing, milestones (create/start/complete/verify), deal timeouts |
| **Payments (points ledger)** | Wallet container + per-deal sub-wallets, deposits/withdrawals, escrow hold/release, transaction approval workflow, fee deduction |
| **Reviews & Disputes** | Multi-dimensional reviews, admin hide, disputes with evidence/responses/admin resolution |
| **Verifications** | Manual verification requests with admin approve/reject/revoke, verification levels feeding trust score |
| **Messaging** | Direct messages, chat rooms, read receipts, reactions, replies, soft delete, admin broadcast, WebSocket endpoint |
| **Trust score** | 8-component weighted calculator, role scores, nightly recalculation worker, lifecycle counter wiring |

## 2. Domain gaps

### 2.1 Party & Profile
- **PartyGroup management** — the PDF defines groups/cooperatives with governance rules, member votes, contribution percentages, and group-level deal approval. Only the `PARTY_GROUP` enum variant exists.
- **KYC provider integration** — manual admin verification exists, but there is no automated KYC adapter (Onfido/Jumio/Plaid), SDK token flow, or inbound KYC webhooks.
- **Rich party profile fields** — `foundedYear`, `teamSize`, `socialLinks`, `bannerImageUrl`, `languages`, etc. are not persisted or exposed.
- **Verification provider metadata** — `provider_reference`, `provider_payload`, document-level statuses, and expiry are not modeled.

### 2.2 Marketplace catalog (Resources, Needs, Enhancements)
The database tables `resources`, `needs`, and `enhancements` exist, but there are **no domain entities, repositories, use cases, or routes** for them.

Missing:
- `Resource` CRUD + search/filter/geo APIs
- `Need` CRUD + search/filter APIs
- `Enhancement` CRUD + search/filter APIs
- Deal-bound resource/need/enhancement linking
- Category tree management (`categories` is only seeded and referenced by ID)
- Full-text / faceted search over the catalog

### 2.3 Matching & Discovery
Only basic party search/nearby exists. The PDF expects a full matching engine:

- Triadic match suggestions (`GET /api/v1/matches`)
- Match requests (`POST /api/v1/matches/request`)
- Accept / decline / counter-propose match
- Compatibility scoring endpoint with multi-dimensional breakdown
- Discovery browsing by domain (`/api/v1/discovery/domains`)
- Public deal discovery / deal suggestion engine
- Match-to-deal conversion flow

### 2.4 Deal lifecycle refinements
- **Deal history endpoint** — history is recorded but not exposed via `GET /api/v1/deals/{id}/history`.
- **Deal cloning** — not implemented.
- **Explicit participation accept/decline/withdraw** — participation status changes implicitly during transition to `NEGOTIATING`; dedicated endpoints are missing.
- **Invitation workflow** — invite-by-email/link for parties not yet on the platform is not implemented.
- **Public / published deals** — `is_public` exists but there is no public discovery/search API.
- **Deal templates** — not implemented.

### 2.5 Negotiation & Value Distribution
- **Distribution models** — the enum has `RevenueShare`, `CostPlus`, `Barter`, `Hybrid`, but the validator and settlement logic only handle the fixed-price path.
- **Market benchmarks & dynamic pricing** — no pricing oracle, surplus analysis, or benchmark premiums.
- **Real-time negotiation feedback** — the validator returns a score, but there is no “negotiation feedback” endpoint with suggested improvements.
- **Advanced term dimensions** — terms are generic text clauses; structured dimensions (price bands, revenue-share tiers, indexed pricing) are not modeled.

### 2.6 Agreements
- **Agreement preview** before locking terms is not exposed.
- **Agreement templates** — only inline text rendering exists.
- **E-signature integration** — signatures are in-app only; no DocuSign/HelloSign adapter or inbound e-sign webhooks.
- **Contract versioning** exists, but version diffing / audit of previous versions is minimal.

### 2.7 Execution & Milestones
- **Smart-contract / automated execution layer** — missing entirely.
- **Oracle integration** — no weather, delivery, quality, price, or IoT oracle feeds.
- **Advanced milestone triggers** — only party/admin consensus exists. Time-based, oracle-based, third-party-verify, and external-event triggers are not supported.
- **Conditional payments** — IF-THEN, graduated, penalty, bonus, and contingent payments are not implemented.
- **Automatic holdback release** — final holdback/release logic after dispute window is not implemented.
- **Force majeure handling** — not implemented.
- **Milestone reassignment** endpoint is missing.

### 2.8 Payments & Settlement
- **Automatic escrow funding** — when a deal moves to `COMMITTED`, the consumer is not prompted or required to fund escrow automatically.
- **Payment provider integration** — no Stripe/bank/crypto adapters or inbound payment webhooks.
- **Refunds / chargebacks / manual refunds** — not implemented.
- **Settlement saga** — the distributed saga pattern (finalize milestones → calculate settlement → release escrow → process settlement → update reputation → close deal → notify) is not modeled as a saga.
- **Multi-currency** — all values are currently `POINTS`; fiat/crypto multi-currency support is missing.

### 2.9 Trust, Reputation & Safety
- **Trust badges** — awarded/revoked based on thresholds; not implemented.
- **Fraud detection signals** — new-account high-value deals, circular patterns, chargeback limits, etc. are not implemented.
- **Review challenge process** — only admin hide/unhide exists.
- **Reputation-based gating** — min trust score to participate in large deals is not enforced.
- **Report issue / block functionality** — not implemented.

### 2.10 Notifications
Only transactional email (verification/password reset) exists. Missing:

- Notification entity, repository, and CRUD API
- Notification preferences / quiet hours / channel selection
- Notification templates (per channel, per locale)
- In-app notification center
- SMS / push delivery channels and providers
- Subscription topics (`deal:{id}`, `party:{id}:matches`, etc.)
- Notification workers and delivery tracking

### 2.11 Messaging enhancements
- **File/media attachments** in messages
- **Message search**
- Typing indicators, drafts, edit history (already noted as future work in `message.md`)

### 2.12 Admin & Moderation
Only scoped admin endpoints exist. Missing:

- `AdminAction` audit log
- Content moderation decisions
- User suspend / reinstate
- Platform configuration CRUD
- Manual refund workflow
- Admin dashboard metrics
- Dispute mediator assignment beyond admin resolution

## 3. Platform services gaps

The PDF decomposes the platform into ~16 services. Hayaland is currently a single monolith with no inter-service communication layer. The following services are missing entirely:

| Service | Why it matters |
|---------|----------------|
| **Notification Service** | Multi-channel delivery, templates, preferences |
| **Search Service** | Full-text / faceted / geo search over deals, parties, resources |
| **Analytics & Reporting Service** | Dashboards, metrics, reports, data snapshots |
| **Media/Document Service** | Uploads, OCR, virus scanning, previews, object storage |
| **Admin & Moderation Service** | Audit logs, moderation decisions, platform config |
| **Resource/Asset Catalog Service** | Standalone resource/need/enhancement marketplace |
| **Matching/Discovery Service** | Triadic match algorithm and discovery APIs |
| **Value Distribution & Pricing Service** | Advanced models, benchmarks, dynamic pricing |
| **Agreement/Contract Service** | Templates, e-sign orchestration |

## 4. Architectural & cross-cutting gaps

### 4.1 Event-driven infrastructure
- **Event bus** — no Kafka/RabbitMQ/SNS; only an in-memory realtime publisher used for tests.
- **Domain event schema** — no standardized event envelope (`event_id`, `correlation_id`, `causation_id`, etc.).
- **Event producers/consumers** — deal lifecycle changes do not publish `deal.created`, `deal.state.changed`, `payment.escrow.released`, `trust.score.updated`, etc.
- **Event sourcing** — only a `deal_history` audit table exists, not a full event store for aggregate reconstruction.

### 4.2 CQRS / read models
- **Elasticsearch** for deal/party/resource search
- **ClickHouse/BigQuery** for analytics
- **Redis** caching for hot reads
- **Projection workers** to build read models from domain events

### 4.3 Distributed transactions
- **Saga orchestration** for deal creation and deal settlement sagas, with compensation steps and saga state store.
- **Dead-letter queue** for failed saga steps.

### 4.4 API surface
- **gRPC inter-service contracts** — not defined.
- **GraphQL schema & endpoint** — not implemented.
- **Webhook management** — inbound payment/KYC/e-sign webhooks and outbound enterprise client webhooks.
- **Idempotency keys** for mutation endpoints are not enforced.
- **Rate limiting** — not implemented.
- **RFC 7807 Problem Details** — partially followed but not fully standardized.

### 4.5 Security & compliance
- **OAuth2/OIDC** authentication flow — only email/password JWT exists.
- **ABAC** — only RBAC scope checks exist.
- **Data encryption at rest** for sensitive fields beyond message encryption.
- **Audit logging** across all admin and payment actions.
- **Security monitoring / threat mitigation** rules.
- **GDPR/data retention** policies and archival.

### 4.6 Observability & resilience
- Distributed tracing across requests/events
- Structured metrics for dashboards
- Circuit breakers / bulkheads for external integrations
- Feature flags
- Caching strategy (Redis)
- Auto-scaling / chaos engineering patterns (documented but not implemented)

## 5. Suggested implementation order

The PDF roadmap is organized in four phases. Mapped to the current codebase:

1. **Finish MVP marketplace** (highest priority)
   - Resource / Need / Enhancement catalog + CRUD/search APIs
   - Category management tree
   - Triadic matching engine + discovery endpoints
   - Notification service (in-app + email templates)
   - Admin audit log + platform config

2. **Harden deal execution & payments**
   - Automatic escrow funding on `COMMITTED`
   - Settlement saga + compensation logic
   - Payment-provider webhooks
   - Agreement templates + e-sign adapter

3. **Add platform intelligence**
   - Search service (Elasticsearch) + analytics read models
   - Dashboards, metrics, reports
   - Advanced value distribution models
   - Fraud detection signals

4. **Scale & split**
   - Event bus (Kafka) + event sourcing for deals
   - gRPC inter-service contracts
   - Microservice decomposition
   - GraphQL gateway
   - Smart-contract / oracle execution layer

## 6. Summary

The Hayaland codebase has a solid, tested **core deal engine**: identity, parties, deal lifecycle, terms, value distribution, agreement signing, milestones, payments ledger, reviews, disputes, verifications, and trust scoring are all wired and passing tests.

To become a full implementation of `3partydeal.pdf`, the main additions are:

- **Marketplace & matching:** resources/needs/enhancements catalog, triadic matching, discovery.
- **Notifications:** preferences, templates, in-app center, SMS/push.
- **Platform services:** search, analytics, media/document, admin/moderation.
- **Architecture:** event bus, event sourcing, CQRS projections, sagas, gRPC, GraphQL, webhooks.
- **Security/compliance:** OAuth2/OIDC, ABAC, audit logging, data retention.
- **Advanced execution:** smart contracts, oracles, conditional payments, e-signatures, advanced distribution models.
