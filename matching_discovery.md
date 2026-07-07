# Matching & Discovery Engine — Implementation Plan

> **Status:** Plan awaiting approval  
> **Scope:** Introduce a knowledge-graph-powered matching and discovery engine for Hayaland.  
> **Target file:** `matching_discovery.md` (to be created on plan approval).  
> **Coverage target:** > 85 % line coverage for all new code.

---

## 1. Executive Summary

Hayaland already has the data needed for algorithmic triadic matching: parties with roles, profiles, locations, trust scores; catalogue items (resources, needs, enhancements); deals with participations; reviews; and a `match_suggestions` table. However, **no code references `match_suggestions`**, and there is no scoring engine, no discovery API beyond domains, and no match-to-deal conversion flow.

This plan proposes a **PostgreSQL-native knowledge graph** that reuses the existing hexagonal stack (Actix Web + sqlx + PostgreSQL) while giving admins graph visibility and control. The engine will:

1. Model the marketplace as a graph with parties, catalogue items, categories, deals, and historical outcomes as nodes/edges.
2. Compute deterministic 7-dimension compatibility scores for candidate Supplier–Consumer–Enhancer triplets.
3. Persist ranked suggestions in `match_suggestions` and expose them via REST.
4. Let parties accept, decline, or counter-propose matches.
5. Convert an accepted match into a `Deal` draft with pre-filled participations.
6. Provide admin endpoints to inspect, reset, and tune the graph and match scores.

The recommended backend is PostgreSQL itself (graph stored as relational tables + CTE traversals), not a separate graph database. This keeps the monolith operationally simple, preserves sqlx offline metadata, and makes the feature deliverable within the current CI pipeline. A dedicated graph database (Neo4j/Memgraph) is noted as a future extraction point but is intentionally out of scope for this milestone.

---

## 2. Current State & Constraints

### 2.1 What already exists

| Layer | Asset | Relevance to matching |
|-------|-------|----------------------|
| Domain | `Party`, `DealRole`, `RoleProfile`, `PartyRole`, `UserPartyMembership` | Core nodes of the graph. |
| Domain | `Deal`, `DealParticipation`, `DealStatus` | Historical deal edges and success signals. |
| Domain | `Resource`, `Need`, `Enhancement` entities + `CatalogRepository` | Catalogue nodes; role-specific intent. |
| Domain | `TrustScore`, `Review`, `Dispute` | Reputation / risk inputs to scoring. |
| Domain | `Category` tree | Domain / type taxonomy for alignment. |
| DB | `match_suggestions` table | Persisted triadic suggestions (unused). |
| DB | PostGIS `GEOGRAPHY` columns & GIST indexes | Geo fit computation. |
| API | `GET /api/v1/discovery/domains`, `/api/v1/discovery/domains/{id}` | Discovery entry points; needs extension. |
| Auth | `X-Party-ID` header, scope helpers | Party-acting model already in place. |
| Architecture | Hexagonal crates (`domain` → `application` → `infrastructure` → `api`) | New code follows existing boundaries. |

### 2.2 Key gaps

- No `MatchSuggestion` domain entity or repository port.
- No `MatchingEngine` application port / use case.
- No code reading or writing `match_suggestions`.
- No match response lifecycle (`ACCEPTED`, `DECLINED`, `COUNTER_PROPOSED`).
- No `GET /api/v1/matches` or `POST /api/v1/matches/{id}/respond` endpoints.
- No match-to-deal conversion.
- No admin graph inspection / reset / tuning APIs.
- No explicit graph tables beyond implicit FK relationships.

### 2.3 Constraints

- Keep the monolith; no new infrastructure services in this milestone.
- Use existing PostgreSQL + sqlx patterns (offline `.sqlx/` metadata).
- Follow dependency direction: `api → application → domain`, `infrastructure → application/domain`.
- All public errors typed with `thiserror`; API maps to HTTP.
- Maintain > 85 % line coverage.
- Migrations must be idempotent and backwards-compatible.
- Do not modify existing source files as part of this planning task.

---

## 3. Goals & Non-Goals

### 3.1 Goals

- Introduce a `MatchSuggestion` aggregate and a `MatchingEngine` port.
- Build a deterministic, explainable 7-dimension compatibility scorer:
  1. Resource/need alignment
  2. Geographic fit
  3. Temporal availability
  4. Trust score
  5. Value alignment
  6. Historical success
  7. Risk profile
- Expose REST APIs:
  - `GET /api/v1/matches` — ranked suggestions for the current party.
  - `POST /api/v1/matches/{id}/respond` — accept / decline / counter-propose.
  - `GET /api/v1/discovery/domains` — already exists; enrich with match counts.
  - `GET /api/v1/discovery/deals` — public deal opportunities.
- Convert accepted matches into `Deal` drafts with pre-filled participations.
- Give admins visibility into the graph and controls to reset/modify scores.

### 3.2 Non-goals

- Real-time streaming / event-driven matching triggers (kept simple; batch or on-demand only).
- Machine-learning scoring; weights are deterministic and configurable.
- Separate graph database in this milestone.
- Multi-thread negotiation inside match counter-proposals (counter-propose updates a few scalar fields only).
- Public deal details beyond sanitized discovery metadata (deal privacy rules remain).

---

## 4. Architecture Decision: PostgreSQL-Native Knowledge Graph

### 4.1 Rationale

The phrase "knowledge graph" describes a structured network of entities and relationships, not a specific product. The existing schema already encodes most of the graph:

- **Nodes:** parties, resources, needs, enhancements, categories, deals.
- **Edges:** `user_party_memberships`, `party_roles`, `deal_participations`, `resources.supplier_party_id`, `needs.consumer_party_id`, `enhancements.enhancer_party_id`, `match_suggestions`, reviews, disputes.

A separate graph database would require:
- New container/service in dev and CI.
- A sync layer from Postgres to the graph DB.
- New query language and driver in the Rust workspace.
- Operational complexity before product/market fit is proven.

By implementing the graph in PostgreSQL we:
- Reuse existing repositories, migrations, and sqlx workflows.
- Use recursive CTEs for graph traversal (e.g. "parties that worked with parties that worked with X").
- Keep the hexagonal boundary clean: the `MatchingEngine` port can later be re-implemented against a graph DB without touching domain/application code.

### 4.2 Graph model

```text
Party (node)
├── HAS_ROLE → PartyRole → SupplierProfile / ConsumerProfile / EnhancerProfile
├── MEMBER_OF → UserPartyMembership → User
├── LOCATED_AT → GeoPoint
├── TRUST_SCORE → TrustScore
├── LISTED → Resource | Need | Enhancement
├── PARTICIPATED_IN → DealParticipation → Deal
├── REVIEWED → Review
├── DISPUTED → Dispute
└── MATCHED_WITH → MatchSuggestion → Party

Category (node)
├── PARENT_OF → Category
├── DOMAIN_FOR → Party.primary_domain_id
├── RESOURCE_TYPE_FOR → Resource
├── NEED_TYPE_FOR → Need
└── ENHANCEMENT_TYPE_FOR → Enhancement

Deal (node)
├── HAS_PARTICIPATION → DealParticipation → Party
├── HAS_TERM → Term
├── HAS_VALUE_DISTRIBUTION → ValueDistribution
└── HAS_HISTORY → DealHistory
```

### 4.3 Graph access patterns

| Query | SQL technique |
|-------|---------------|
| Find suppliers for a consumer need | Join `resources` → `categories` (descendants) ← `needs`. |
| Find enhancers for a supplier-consumer pair | Join `enhancements` by `enhancement_type_id` and geo radius. |
| Prior successful triplets | Aggregate `deal_participations` + `deals.deal_status = 'COMPLETED'`. |
| Mutual connections / 2nd-degree | Recursive CTE over `deal_participations` grouped by party pairs. |
| Admin graph stats | Count nodes/edges per type, average degree, score distribution. |

---

## 5. Domain Layer Additions (`crates/domain`)

### 5.1 New entity: `MatchSuggestion`

```rust
pub struct MatchSuggestion {
    pub id: Uuid,
    pub supplier_party_id: Uuid,
    pub consumer_party_id: Uuid,
    pub enhancer_party_id: Uuid,
    pub match_status: MatchStatus,
    pub match_score: f64,
    pub score_breakdown: MatchScoreBreakdown,
    pub match_reason: String,
    pub resource_category_id: Option<Uuid>,
    pub need_category_id: Option<Uuid>,
    pub enhancement_category_id: Option<Uuid>,
    pub suggested_deal_value: Option<Decimal>,
    pub generated_by: MatchGeneratedBy,
    pub expires_at: Option<OffsetDateTime>,
    pub converted_deal_id: Option<Uuid>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

Enums:

- `MatchStatus::Pending`, `Accepted`, `Declined`, `CounterProposed`, `Expired`, `ConvertedToDeal`.
- `MatchGeneratedBy::Algorithm`, `PlatformAdmin`, `UserReferral`.

### 5.2 New value object: `MatchScoreBreakdown`

```rust
pub struct MatchScoreBreakdown {
    pub resource_need_alignment: f64,
    pub geographic_fit: f64,
    pub temporal_availability: f64,
    pub trust_score: f64,
    pub value_alignment: f64,
    pub historical_success: f64,
    pub risk_profile: f64,
    pub weights: MatchScoreWeights,
}
```

Weights default to the PDF specification and are overridable per request / admin config:

| Dimension | Weight |
|-----------|--------|
| Resource/need alignment | 0.25 |
| Value alignment | 0.20 |
| Trust score | 0.15 |
| Geographic fit | 0.10 |
| Temporal availability | 0.10 |
| Historical success | 0.10 |
| Risk profile | 0.10 |

### 5.3 New repository port: `MatchRepository`

```rust
#[async_trait]
pub trait MatchRepository: Send + Sync {
    async fn create(&self, suggestion: &MatchSuggestion) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<MatchSuggestion>, DomainError>;
    async fn list_for_party(
        &self,
        party_id: Uuid,
        role: Option<DealRole>,
        filters: &MatchFilters,
    ) -> Result<Vec<MatchSuggestion>, DomainError>;
    async fn update_status(&self, id: Uuid, status: MatchStatus, notes: Option<String>) -> Result<(), DomainError>;
    async fn update_counter_proposal(
        &self,
        id: Uuid,
        value: Option<Decimal>,
        notes: Option<String>,
    ) -> Result<(), DomainError>;
    async fn set_converted_deal(&self, id: Uuid, deal_id: Uuid) -> Result<(), DomainError>;
    async fn delete_by_party(&self, party_id: Uuid) -> Result<u64, DomainError>;
    async fn delete_all(&self) -> Result<u64, DomainError>;
    async fn count_by_status(&self, party_id: Uuid) -> Result<MatchCountByStatus, DomainError>;
}
```

### 5.4 Domain errors

Add to `DomainError`:

- `MatchNotFound`
- `InvalidMatchStatus { message }`
- `InvalidMatchResponse { message }`
- `MatchExpired`
- `PartyNotMatchParticipant`

### 5.5 Domain invariants

- A `MatchSuggestion` always references three distinct parties.
- Only parties referenced by a suggestion may respond to it.
- A suggestion in terminal status (`ConvertedToDeal`, `Expired`, `Declined`) cannot be responded to.
- `match_score` is in `[0.0, 1.0]`.
- Score breakdown components are also in `[0.0, 1.0]`.

---

## 6. Application Layer Additions (`crates/application`)

### 6.1 New outbound port: `MatchingEngine`

```rust
#[async_trait]
pub trait MatchingEngine: Send + Sync {
    /// Find ranked triadic matches for a given party acting in a given role.
    async fn find_matches(
        &self,
        party_id: Uuid,
        role: DealRole,
        filters: &MatchQuery,
    ) -> Result<Vec<MatchSuggestion>, ApplicationError>;

    /// Score an arbitrary triplet without persisting a suggestion.
    async fn score_triplet(
        &self,
        supplier_party_id: Uuid,
        consumer_party_id: Uuid,
        enhancer_party_id: Uuid,
    ) -> Result<MatchSuggestion, ApplicationError>;

    /// Regenerate all pending suggestions for a party (admin / refresh).
    async fn regenerate_for_party(&self, party_id: Uuid) -> Result<u64, ApplicationError>;

    /// Reset all suggestions (admin).
    async fn reset_all(&self) -> Result<u64, ApplicationError>;
}
```

### 6.2 New use cases

| File | Responsibility |
|------|----------------|
| `matching/find_matches.rs` | Orchestrate `MatchingEngine::find_matches`, persist generated suggestions, return DTOs. |
| `matching/respond_to_match.rs` | Validate caller, update match status, handle accept/decline/counter. |
| `matching/convert_match_to_deal.rs` | When all three parties accepted, create a `Deal` draft with participations. |
| `matching/get_match.rs` | Fetch a single suggestion with score breakdown. |
| `matching/admin_reset_matches.rs` | Clear suggestions for a party or globally. |
| `matching/admin_recalculate_scores.rs` | Trigger score recalculation for a triplet or party. |
| `discovery/list_public_deals.rs` | Sanitized list of `is_public = true` deals. |

### 6.3 DTOs

- `FindMatchesQuery` — role, domain, geo, minScore, maxResults, dealValue range.
- `MatchResponseCommand` — match_id, response, notes, optional counter value.
- `MatchResult` — suggestion + participant summaries + distance + score breakdown.
- `PublicDealResult` — public deal discovery projection.
- `MatchGraphStats` — admin graph statistics.

### 6.4 Application errors

Add to `ApplicationError`:

- `MatchNotFound`
- `MatchExpired`
- `InvalidMatchResponse`
- `PartyNotMatchParticipant`
- `MatchAlreadyResponded`
- `MatchingEngineUnavailable` (fallback)

---

## 7. Infrastructure Layer (`crates/infrastructure`)

### 7.1 New repository: `PostgresMatchRepository`

Implements `MatchRepository` with `sqlx::query!` macros. Operations:

- Insert suggestion.
- Select by ID.
- Select list filtering by `supplier_party_id OR consumer_party_id OR enhancer_party_id`, status, role-specific columns, score, pagination.
- Update status / counter-proposal / converted deal.
- Admin bulk delete.

### 7.2 New engine: `SqlMatchingEngine`

The concrete `MatchingEngine` implementation. Algorithm:

1. **Load the actor party** and its active role profile.
2. **Determine missing roles.** If actor is supplier, find consumers + enhancers; etc.
3. **Fetch candidate catalogue items**:
   - For supplier actor: active needs within domain/geo/temporal filters.
   - For consumer actor: active resources within domain/geo/temporal filters.
   - For enhancer actor: active resources + needs that need enhancement.
4. **Build candidate triplets** from items, avoiding duplicates and parties already in active deals with the actor.
5. **Score each triplet** using the 7-dimension pure function (see §9).
6. **Rank, diversify, and cap** results (max 3 triads sharing the same supplier; default top 10, max 50).
7. **Persist suggestions** with `PENDING` status, or refresh existing rows on regeneration.

### 7.3 Graph helper queries

Implement as reusable SQL functions or CTEs:

- `party_success_partners(party_id)` — parties that completed a deal with this party.
- `party_second_degree_partners(party_id)` — 2nd-degree successful connections.
- `category_descendants(category_id)` — recursive category tree (already used in discovery).
- `deal_success_rate(party_id, role)` — completed / started ratio per role.

### 7.4 Offline metadata

Run `cargo sqlx prepare --workspace` after adding queries and commit `.sqlx/` JSON files.

---

## 8. API Layer (`crates/api`)

### 8.1 New routes

```rust
// crates/api/src/routes/matching.rs
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/matches")
            .route(web::get().to(handlers::matching::list_matches))
            .route(web::post().to(handlers::matching::request_matches)), // on-demand regeneration
    )
    .service(
        web::resource("/matches/{id}")
            .route(web::get().to(handlers::matching::get_match)),
    )
    .service(
        web::resource("/matches/{id}/respond")
            .route(web::post().to(handlers::matching::respond_to_match)),
    )
    .service(
        web::resource("/matches/{id}/convert-to-deal")
            .route(web::post().to(handlers::matching::convert_match_to_deal)),
    )
    .service(
        web::resource("/admin/matches")
            .route(web::get().to(handlers::matching::admin_list_matches))
            .route(web::delete().to(handlers::matching::admin_reset_matches)),
    )
    .service(
        web::resource("/admin/matches/stats")
            .route(web::get().to(handlers::matching::admin_graph_stats)),
    )
    .service(
        web::resource("/admin/matches/recalculate")
            .route(web::post().to(handlers::matching::admin_recalculate)),
    )
    // Discovery
    .service(
        web::resource("/discovery/deals")
            .route(web::get().to(handlers::discovery::list_public_deals)),
    );
}
```

Wire the module in `crates/api/src/routes/mod.rs`.

### 8.2 Auth & scope model

| Endpoint | Required scope / role |
|----------|----------------------|
| `GET /api/v1/matches` | `parties:read` or `admin:parties` (uses `X-Party-ID`) |
| `POST /api/v1/matches` | `parties:write` or `admin:parties` |
| `GET /api/v1/matches/{id}` | Participant of the match or `admin:parties` |
| `POST /api/v1/matches/{id}/respond` | Participant of the match or `admin:parties` |
| `POST /api/v1/matches/{id}/convert-to-deal` | Participant of the match or `admin:deals` |
| `GET /api/v1/discovery/deals` | Public (no auth) |
| `GET /api/v1/discovery/domains` | Public (existing) |
| `GET /api/v1/admin/matches*` | `admin:parties` or `admin:*` |

### 8.3 DTOs

Create `crates/api/src/dto/matching.rs`:

- `MatchesQuery` (query params with validator).
- `MatchResponseRequest`.
- `MatchResponse`, `MatchScoreBreakdownResponse`, `MatchesListResponse`.
- `PublicDealListResponse`.
- `AdminMatchStatsResponse`.

### 8.4 Handler responsibilities

- Resolve `X-Party-ID` using existing helpers.
- Validate query/body with `validator`.
- Enforce scopes.
- Map application results to HTTP 200/201/404/403/409/422.
- On accept: if all three parties have accepted, allow conversion to deal.

---

## 9. Compatibility Scoring Algorithm

### 9.1 Overall score

```text
SCORE = Σ (dimension_i × weight_i)
```

Default weights from the PDF:

```rust
pub const DEFAULT_WEIGHTS: MatchScoreWeights = MatchScoreWeights {
    resource_need_alignment: 0.25,
    value_alignment: 0.20,
    trust_score: 0.15,
    geographic_fit: 0.10,
    temporal_availability: 0.10,
    historical_success: 0.10,
    risk_profile: 0.10,
};
```

### 9.2 Dimension implementations

#### 1. Resource/Need Alignment (0.25)

Inputs:
- Resource type vs need category (exact descendant = 1.0, same domain = 0.7, convertible = 0.4, mismatch = 0.0).
- Quantity ratio: `1 - |supply - demand| / max(supply, demand)`.
- Quality alignment: profile quality standard match.
- Capacity utilization: optimal zone 60–90 % = 1.0.

Formula: average of available sub-scores, clamped to [0, 1].

#### 2. Value Alignment (0.20)

Inputs:
- Budget overlap between consumer budget and supplier opportunity cost + enhancer rate.
- Payment terms compatibility from role profiles.
- Gini-inspired fairness of implied shares (placeholder until value distribution exists).

#### 3. Trust Score (0.15)

Inputs from `trust_scores`:
- Overall score normalized to [0, 1].
- Role-specific score for the role the party would play.
- Verification level.
- Dispute rate.

Use the lowest trust score among the three parties to avoid dragging down a good triad.

#### 4. Geographic Fit (0.10)

Use PostGIS `ST_Distance`:

| Distance | Score |
|----------|-------|
| < 10 km | 1.0 |
| < 50 km | 0.9 |
| < 200 km | 0.7 |
| < 1000 km | 0.5 |
| > 1000 km | 0.3 |

If `service_radius_km` is set, require the target to be within it; otherwise degrade gracefully.

#### 5. Temporal Availability (0.10)

Inputs:
- Availability windows from resource/need/enhancement profiles.
- Jaccard index of overlapping windows.
- Urgency match from priority fields.

#### 6. Historical Success (0.10)

Inputs:
- Prior completed deals together (+0.2 per deal, max 1.0).
- Completed deals of similar category (+0.8 if any).
- Shared successful partners via 2nd-degree CTE (+0.1 each, max 0.3).

#### 7. Risk Profile (0.10)

Inputs:
- Active deal count per party (high load = lower score).
- Cancellation / dispute ratio.
- Deal size comfort: suggested value vs party’s historical completed value range.

### 9.3 Score thresholds

| Score range | Interpretation |
|-------------|----------------|
| 0.90 – 1.00 | Excellent |
| 0.70 – 0.89 | Good |
| 0.50 – 0.69 | Fair |
| < 0.50 | Poor — not suggested unless no better candidates |

Minimum suggestion threshold: 0.50 (configurable).

### 9.4 Explainability

Every suggestion includes:

- Total `match_score`.
- `score_breakdown` with all 7 dimension scores.
- Human-readable `match_reason` string generated from the strongest dimensions.
- Distance in km between parties.
- Common domain/category names.

---

## 10. Match Response Lifecycle

### 10.1 States

```text
PENDING
  ├── ACCEPTED (by one party)
  │     └── When all three ACCEPTED → eligible for convert-to-deal
  ├── DECLINED (by any party) → terminal
  ├── COUNTER_PROPOSED (by one party)
  │     └── Other parties see counter and can ACCEPT / DECLINE / COUNTER
  └── EXPIRED (after expires_at) → terminal
```

### 10.2 Counter-propose semantics

Counter-propose updates:
- `match_status = COUNTER_PROPOSED`
- `suggested_deal_value` (optional)
- `counter_notes` (optional)
- `updated_at`

It does **not** version the suggestion; a counter resets the response accumulator (other parties must respond again). This keeps the implementation simple.

### 10.3 Convert to deal

When all three parties have accepted:

1. Call `CreateDeal` use case with:
   - `actor_user_id` = caller.
   - `actor_party_id` = caller's party.
   - `consumer_party_id`, `supplier_party_id`, `enhancer_party_id` from the suggestion.
   - `domain_category_id` from suggestion.
   - Title generated from category + parties.
2. On success, update `match_suggestions.converted_deal_id` and `match_status = CONVERTED_TO_DEAL`.
3. Return the new deal ID.

---

## 11. Admin Graph Capabilities

### 11.1 Endpoints

| Endpoint | Purpose |
|----------|---------|
| `GET /api/v1/admin/matches` | List all suggestions with filters (status, party, score). |
| `DELETE /api/v1/admin/matches?party_id={id}` | Delete suggestions for a party; omit `party_id` to delete all. |
| `GET /api/v1/admin/matches/stats` | Graph stats: node/edge counts, suggestion counts, score distribution. |
| `POST /api/v1/admin/matches/recalculate` | Recalculate scores for a party or globally. |
| `PATCH /api/v1/admin/matches/{id}/score` | Manually adjust a suggestion's score and reason (audit logged). |

### 11.2 Graph visualization data

`GET /api/v1/admin/matches/stats` returns:

```json
{
  "node_counts": {
    "parties": 120,
    "resources": 45,
    "needs": 30,
    "enhancements": 25,
    "deals": 80,
    "categories": 18
  },
  "edge_counts": {
    "party_roles": 140,
    "deal_participations": 240,
    "catalog_items": 100,
    "match_suggestions": 350
  },
  "match_status_counts": {
    "pending": 200,
    "accepted": 50,
    "declined": 60,
    "converted_to_deal": 40
  },
  "score_distribution": {
    "0.0-0.49": 10,
    "0.5-0.69": 80,
    "0.7-0.89": 180,
    "0.9-1.0": 80
  }
}
```

A future endpoint `GET /api/v1/admin/graph/edges` can return adjacency lists for frontend graph renderers.

### 11.3 Reset / modify controls

- **Reset party:** clear all suggestions involving a party so the engine regenerates with fresh data.
- **Reset all:** truncate `match_suggestions` (cascades no other tables).
- **Modify score:** admin can boost or penalize a specific suggestion with a `manual_override` flag and reason stored in `match_reason`.

### 11.4 Audit logging

Every admin mutation on matches writes to a new `admin_actions` table (or `match_suggestions_audit` table if a full admin audit log is not yet built). The row records:

- admin_user_id
- action_type (RESET, RECALCULATE, MANUAL_SCORE)
- target_match_id (optional)
- before_snapshot / after_snapshot (JSONB)
- reason
- created_at

---

## 12. Database Schema Additions

### 12.1 Additions to `match_suggestions`

The table exists. Extend it with:

```sql
ALTER TABLE match_suggestions
  ADD COLUMN IF NOT EXISTS score_breakdown JSONB NOT NULL DEFAULT '{}',
  ADD COLUMN IF NOT EXISTS counter_notes TEXT,
  ADD COLUMN IF NOT EXISTS responded_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- Composite index for listing pending matches for a party.
CREATE INDEX IF NOT EXISTS idx_match_suggestions_party_status
  ON match_suggestions(supplier_party_id, match_status, match_score DESC);
CREATE INDEX IF NOT EXISTS idx_match_suggestions_party_status_consumer
  ON match_suggestions(consumer_party_id, match_status, match_score DESC);
CREATE INDEX IF NOT EXISTS idx_match_suggestions_party_status_enhancer
  ON match_suggestions(enhancer_party_id, match_status, match_score DESC);

-- Score-based discovery index.
CREATE INDEX IF NOT EXISTS idx_match_suggestions_score
  ON match_suggestions(match_score DESC)
  WHERE match_status = 'PENDING';
```

### 12.2 Optional graph analytics table

```sql
CREATE TABLE IF NOT EXISTS match_graph_snapshots (
    id UUID PRIMARY KEY,
    snapshot_type TEXT NOT NULL, -- 'DAILY', 'ADMIN_TRIGGERED'
    node_counts JSONB NOT NULL,
    edge_counts JSONB NOT NULL,
    score_distribution JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 12.3 Admin audit table (if not using a generic `admin_actions` table)

```sql
CREATE TABLE IF NOT EXISTS match_suggestion_audit_log (
    id UUID PRIMARY KEY,
    admin_user_id UUID REFERENCES users(id),
    action_type TEXT NOT NULL,
    match_suggestion_id UUID REFERENCES match_suggestions(id) ON DELETE SET NULL,
    party_id UUID REFERENCES parties(id) ON DELETE SET NULL,
    before_snapshot JSONB,
    after_snapshot JSONB,
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

---

## 13. Testing Strategy (> 85 % Coverage)

### 13.1 Domain unit tests

- `MatchSuggestion::new` enforces distinct parties.
- `MatchScoreBreakdown` clamps scores and computes weighted total.
- `MatchStatus` transitions and terminal states.
- `MatchGeneratedBy` serialization.

### 13.2 Application unit tests (fake repositories)

- `FindMatches` returns ranked suggestions.
- `RespondToMatch` allows only participants.
- `RespondToMatch` rejects expired / terminal suggestions.
- Counter-propose resets other responses.
- `ConvertMatchToDeal` creates a deal and updates suggestion status.
- Admin reset/recalculate flows.

### 13.3 Infrastructure integration tests

- `PostgresMatchRepository` CRUD and filters.
- `SqlMatchingEngine` scoring against seeded parties/resources/needs.
- Geo scoring uses actual PostGIS distance.
- Graph CTE helper queries return correct 2nd-degree partners.

### 13.4 API tests (`#[sqlx::test]`)

- Anonymous `GET /api/v1/discovery/deals` returns public deals.
- Authenticated `GET /api/v1/matches` returns suggestions for `X-Party-ID`.
- Non-participant cannot respond to a match (403).
- Accept flow + convert-to-deal creates a draft deal.
- Decline marks suggestion terminal.
- Admin can delete/recalculate matches.
- Admin stats endpoint returns expected counts.

### 13.5 Coverage gate

Add the new modules to CI coverage measurement. Target:

- `crates/domain/src/entities/match_suggestion.rs` — > 90 %
- `crates/application/src/matching/` — > 85 %
- `crates/infrastructure/src/matching/` — > 85 %
- `crates/api/src/handlers/matching.rs` — > 85 %

---

## 14. Implementation Order

### Phase 1 — Domain & repository scaffolding

1. Add `MatchSuggestion`, `MatchStatus`, `MatchGeneratedBy`, `MatchScoreBreakdown`, `MatchScoreWeights` to `crates/domain/src/entities/`.
2. Add `MatchRepository` port in `crates/domain/src/repositories/`.
3. Add domain errors (`MatchNotFound`, etc.).
4. Migration: extend `match_suggestions` with `score_breakdown`, `counter_notes`, `responded_at`, `updated_at`, indexes.
5. Implement `PostgresMatchRepository`.
6. Add fake repository to `test_helpers.rs`.

### Phase 2 — Matching engine & scoring

7. Add `MatchingEngine` port in `crates/application/src/ports.rs`.
8. Implement `SqlMatchingEngine` with the 7-dimension scorer.
9. Add graph helper SQL functions/CTEs.
10. Add unit/integration tests for scoring.

### Phase 3 — Application use cases

11. Implement `FindMatches`, `RespondToMatch`, `ConvertMatchToDeal`, `GetMatch`.
12. Implement admin use cases: `AdminResetMatches`, `AdminRecalculateMatches`, `AdminGetGraphStats`.
13. Wire `DomainEventPublisher` to emit `MatchCreated`, `MatchResponded`, `MatchConvertedToDeal` events.

### Phase 4 — API layer

14. Create `crates/api/src/routes/matching.rs`.
15. Create handlers and DTOs.
16. Wire routes in `routes/mod.rs` and add use cases to `AppState` / `main.rs`.
17. Extend `GET /api/v1/discovery/domains` with match counts (optional enrichment).
18. Add `GET /api/v1/discovery/deals`.

### Phase 5 — Admin & polish

19. Add admin endpoints, audit log writes.
20. Add graph snapshot table/endpoint.
21. Add OpenAPI / doc comments to handlers.
22. Run `cargo sqlx prepare --workspace`.
23. Full test pass + coverage gate.

---

## 15. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Score computation becomes slow with many parties | Cap candidates with filters (domain, geo radius); add indexes; profile and paginate. |
| `.sqlx/` metadata drift | Run `cargo sqlx prepare --workspace` after every migration batch. |
| Match scoring feels opaque | Always return `score_breakdown` and `match_reason`; admin can override. |
| Counter-propose versioning complexity | Keep a single mutable suggestion; reset responses on counter. Document limitation. |
| Geo scoring without PostGIS in tests | CI already uses PostgreSQL with PostGIS; local tests rely on `sqlx::test` against the managed cluster. |
| Admin reset deletes useful suggestions | Reset only `PENDING` by default; full reset requires explicit `?all=true`. |

---

## 16. Open Questions / Decisions

1. **Match generation strategy:** Recommended is a hybrid — pre-compute suggestions when catalogue items change or via nightly worker, then re-rank live on `GET /api/v1/matches`. This balances freshness with query speed.
2. **Counter-propose versioning:** Single mutable row with response reset. If multi-round negotiation is needed later, introduce a `match_suggestion_versions` table.
3. **Admin audit log:** Reuse a generic `admin_actions` table if built; otherwise use `match_suggestion_audit_log` as a dedicated table.
4. **Graph visualization:** Start with JSON stats; adjacency-list endpoint can be added as a follow-up for frontend graph renderers.
5. **Discovery deals privacy:** Only `is_public = true` deals are listed, and the response excludes negotiation details, terms, and private party data.

---

## 17. Success Criteria

- [ ] `MatchSuggestion` domain entity, repository port, and Postgres implementation exist and are tested.
- [ ] `MatchingEngine` port and `SqlMatchingEngine` produce ranked triadic suggestions using the 7-dimension scorer.
- [ ] `GET /api/v1/matches` returns suggestions for the authenticated `X-Party-ID`.
- [ ] `POST /api/v1/matches/{id}/respond` supports accept / decline / counter-propose.
- [ ] `GET /api/v1/discovery/deals` returns public deal opportunities.
- [ ] `POST /api/v1/matches/{id}/convert-to-deal` creates a draft deal with pre-filled participations.
- [ ] Admin endpoints allow listing, resetting, recalculating, and manual score adjustment.
- [ ] Admin graph stats endpoint returns node/edge/score distributions.
- [ ] All new code achieves > 85 % line coverage.
- [ ] `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, and `cargo sqlx prepare --workspace --check` pass.
