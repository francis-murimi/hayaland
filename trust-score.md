# Trust Score & Transactions — Hayaland 3-Party Deal Platform

> **Scope:** This document specifies how Hayaland computes a **Trust Score** for every Party from historical deal outcomes, reviews, verification, and platform behaviour. It also documents the existing **Transactions** ledger that records point movements tied to deals.
>
> **Audience:** Backend engineers, product owners, and API consumers.
>
> **Based on:** `3partydeal.pdf` Software Design Document, `hayaland-deal-plan.md`, `deal-plan.md`, `party-guide.md`, `negotiation-guide.md`, `AGENTS.md`, and the existing `hayaland` Rust codebase.
>
> **Implementation status:** The trust-score *inputs* (reviews, disputes, verifications, deal lifecycle, payments, messages) are already implemented. The trust-score *calculation module* and read API are **not yet implemented** — this document is the specification for that work.
>
> **Critical implementation note:** Do **not** build the 8-component calculator first. Several counters and metrics it depends on (`deals_completed_count`, `deals_cancelled_count`, `profile_completeness`, `average_response_hours`, `longevity_days`) are currently never populated. Wire the lifecycle events first, then build the calculator on top of real data.

---

## 1. Goals

1. **Reputation is data-driven.** Every Party's trust score is derived from observable facts: completed/cancelled/disputed deals, multi-dimensional reviews, verification level, response speed, profile completeness, and platform longevity.
2. **Role-aware scoring.** A Party can be a Supplier in one deal and a Consumer in another; trust is tracked per role (`as_supplier_score`, `as_consumer_score`, `as_enhancer_score`) as well as overall.
3. **Transparent, auditable formula.** The calculation weights and inputs are stored in `trust_scores.calculation_formula` so the score can be explained and replayed.
4. **Point ledger mirrors real settlement.** All value exchange on the platform is recorded in platform points. A `Transaction` represents a point movement that mirrors a physical settlement performed outside the platform.
5. **Multi-party approval.** Every transaction that moves points between Parties requires approval from every involved Party before it becomes `VERIFIED`/`COMPLETE`.

---

## 2. Trust Score at a Glance

| Property | Value |
|---|---|
| Internal scale | `0 – 100` points |
| Public display scale | `0.00 – 5.00` stars (API may expose `scoreOutOf5 = overallScore / 20`) |
| Tiers | Bronze `0–39`, Silver `40–59`, Gold `60–74`, Platinum `75–100` |
| Update frequency | Event-driven (deal completion, review submission, verification change, profile update) plus a nightly decay/recalculation job |
| Storage | `trust_scores` table, one row per Party |

A Party's trust score affects:

- **Matching priority** — higher scores are ranked earlier in `GET /api/v1/matches`.
- **Win-Win-Win validation** — high-trust parties reduce the opportunity-cost penalty in the validator.
- **Platform fee tier** — trusted Partners/Champions receive lower per-deal platform fees (future fee-policy feature).
- **Deal eligibility** — parties with too many disputes/cancellations may be blocked from initiating deals.

---

## 3. Implementation Roadmap

This section defines the recommended order of work. It mirrors the dependency chain: the calculator needs live data, so the data-producing lifecycle events must be wired before the calculator is switched on.

### 3.1 Phase 1 — Wire trust-data producers (prerequisites)

Before the calculator exists, make sure every trust input is persisted:

1. **Deal completion/cancellation counters**
   - Increment `trust_scores.deals_completed_count` for all participating parties when `ExecuteTransition` reaches `Completed`.
   - Increment `trust_scores.deals_cancelled_count` for all participating parties when `ExecuteTransition` reaches `Cancelled`.
   - Do the same for timeout-driven cancellations/expirations in `ProcessDealTimeouts`.
   - Update `parties.total_deals_completed` / `total_deals_initiated` consistently.

2. **Timeout / no-show penalty tracking**
   - Decide on storage: either add `timeouts_count` / `no_shows_count` to `trust_scores`, or derive them from `deal_history` events.
   - Populate them in `process_timeouts.rs` and any no-show detection flow.

3. **Profile completeness**
   - Implement a `ProfileCompletenessCalculator`.
   - Call it from `CreateParty`, `UpdateParty`, `AddPartyRole`, `RemovePartyRole`.
   - Write the result to `trust_scores.profile_completeness`.

4. **Response rate**
   - Implement a query/service that computes `average_response_hours` from `messages` and `message_reads`.
   - Run it inside `RecalculateTrustScore` (recommended) or incrementally in message handlers.

5. **Guarantee `trust_scores` row existence**
   - Either create the row in `CreateParty` or make `RecalculateTrustScore` upsert it.

### 3.2 Phase 2 — Build the trust-score module

1. **Domain layer**
   - `crates/domain/src/entities/trust_score.rs` — `TrustScore`, `TrustTier`, component value objects.
   - `crates/domain/src/repositories/trust_score_repository.rs` — read/upsert port.
   - `crates/domain/src/services/trust_calculator.rs` — pure function implementing the 8-component weighted formula.

2. **Application layer**
   - `crates/application/src/trust_scores/recalculate_trust_score.rs` — orchestrates input gathering, calculation, persistence, and public-cache sync.
   - `crates/application/src/trust_scores/get_trust_score.rs` — read use case.
   - `crates/application/src/trust_scores/dto.rs` — DTOs.

3. **Infrastructure layer**
   - `crates/infrastructure/src/repositories/postgres_trust_score_repository.rs` — SQL read/upsert.

4. **API layer**
   - `crates/api/src/routes/trust_scores.rs`.
   - `crates/api/src/handlers/trust_scores/get_trust_score.rs`.
   - `crates/api/src/handlers/trust_scores/recalculate_trust_score.rs`.

5. **Replace the no-op stub**
   - Implement `TrustScoreRecalculationService` wrapping `RecalculateTrustScore`.
   - Swap `NoOpTrustScoreRecalculation` for it in `crates/api/src/main.rs` and test helpers.

### 3.3 Phase 3 — Background jobs & polish

1. **Nightly decay/recalculation job**
   - Recompute `longevity_days`.
   - Apply inactivity decay after 180 days.
   - Expire penalties at 12/24 months.
   - Recalculate response-rate and review-recency weights.

2. **Role-specific scores**
   - Compute `as_supplier_score`, `as_consumer_score`, `as_enhancer_score` using role-scoped deal and review queries.

3. **Win-Win-Win validator fix**
   - Load real `parties.trust_score` into `PartyValidationSnapshot` instead of defaults.

### 3.4 Phase 4 — Deferrable work

These are real gaps but not blockers for the first trust-score release:

- **Matching engine** — uses trust, but trust does not depend on it.
- **PartyGroup governance** — groups can be treated as parties today.
- **Automated email/phone verification** — admin approval already works.
- **Community contribution score** — weight is only 5% and no inputs exist.
- **Trust-tier-based escrow percentages** — affects payments, not score calculation.
- **Score history / time-series API** — nice-to-have UX feature.

---

## 4. Current Implementation Status

| Capability | Status | Location / notes |
|---|---|---|
| Reviews (submit, list, moderate, dimensions) | **Implemented** | `crates/application/src/reviews/`, `crates/api/src/handlers/reviews/`, `crates/domain/src/entities/review.rs` |
| Disputes (raise, respond, resolve, severity, outcome) | **Implemented** | `crates/application/src/disputes/`, `crates/api/src/handlers/disputes/`, `migrations/20260615160000_create_disputes.sql` |
| Party verifications (create, approve, reject, revoke, points → levels) | **Implemented** | `crates/application/src/verifications/`, `crates/domain/src/entities/party_verification.rs` |
| Deal lifecycle & state machine | **Implemented** | `crates/application/src/deals/execute_transition.rs`, `crates/domain/src/entities/deal.rs` |
| Milestones & escrow-release creation | **Implemented** | `crates/application/src/milestones/verify_milestone.rs` |
| Payments / wallets / transactions / approvals | **Implemented** | `crates/application/src/payments/`, `crates/api/src/routes/payments.rs` |
| Messages / conversations / read receipts | **Implemented** | `crates/application/src/messages/`, `migrations/20260615145000_create_messages.sql` |
| Win-Win-Win validator | **Implemented** | `crates/domain/src/services/win_win_win_validator.rs`, `crates/application/src/deals/validate_deal.rs` |
| Trust-score recalculation port | **Implemented as a stub** | `crates/application/src/ports.rs` (`TrustScoreRecalculationPort`, `NoOpTrustScoreRecalculation`) |
| Trust-score row creation | **Partial** | Created lazily by `approve_verification` and `raise_dispute`; **not** created on party creation |
| `trust_scores.deals_completed_count` | **Missing** | Not incremented on `Completed` transition |
| `trust_scores.deals_cancelled_count` | **Missing** | Not incremented on `Cancelled` transition |
| Timeout / no-show penalty tracking | **Missing** | Only `deal_history` log exists |
| `trust_scores.profile_completeness` | **Missing** | Not computed from party/role fields |
| `trust_scores.average_response_hours` | **Missing** | Not computed from messages |
| `trust_scores.longevity_days` | **Missing** | Not computed |
| Trust-score calculation use case | **Missing** | Only the no-op stub is wired |
| Trust-score read API (`GET /api/v1/parties/{id}/trust`) | **Missing** | No routes or handlers exist |
| Synchronisation of `trust_scores.overall_score` → `parties.trust_score` | **Missing** | `parties.trust_score` remains at its default `0.0` |

> **Consequence today:** Reviews, disputes, and verifications all call `TrustScoreRecalculationPort::request_recalculation`, but the call returns immediately without doing any work. The public `parties.trust_score` is always `0.0`, so the Win-Win-Win validator uses default trust values.

---

## 5. Data Model & Scale

### 5.1 Two trust values

The codebase stores two related trust values:

| Value | Column | Scale | Purpose |
|---|---|---|---|
| Cached public score | `parties.trust_score` | `0 – 100` (or mapped to `0 – 5` in API) | Used by party search, list, and nearby endpoints. |
| Internal source of truth | `trust_scores.overall_score` | `0 – 100` | Maintained by the trust-score module; recalculated on events. |

The public value is a **read-only cache** of the internal value. When the trust-score module is implemented, recalculation must end with:

```rust
party.trust_score = trust_score.overall_score;
```

> **Current state:** `parties.trust_score` is never updated, so it stays at `0.0`.

### 5.2 Public 0–5 mapping

API consumers may receive either scale. The canonical mapping is:

```text
score_out_of_5 = round(overall_score / 20, 2)
```

Example: `overall_score = 78` → `3.90` stars.

### 5.3 `trust_scores` column ownership

| Column | Populated by | Used for |
|---|---|---|
| `overall_score` | Recalculation | Final public trust score cache. |
| `as_supplier_score` | Recalculation | Role-specific score. |
| `as_consumer_score` | Recalculation | Role-specific score. |
| `as_enhancer_score` | Recalculation | Role-specific score. |
| `deals_completed_count` | Deal lifecycle | Transaction history component. |
| `deals_cancelled_count` | Deal lifecycle | Transaction history / penalty component. |
| `deals_disputed_count` | `RaiseDispute` / `ResolveDispute` | Dispute history component. |
| `average_response_hours` | Response-rate tracker | Response-rate component. |
| `profile_completeness` | Profile update handler | Profile completeness component. |
| `verification_level` | `ApproveVerification` | Verification component. |
| `longevity_days` | Nightly job / recalculation | Longevity component. |
| `calculation_formula` | Recalculation | Audit / transparency JSONB. |
| `last_calculated_at` / `next_calculation_at` | Recalculation | Cache metadata. |

---

## 6. Trust Inputs & Integration Seam

### 6.1 Integration port

All trust-impacting use cases depend on the outbound port defined in `crates/application/src/ports.rs`:

```rust
/// Outbound port used to request trust-score recalculation when a trust input changes.
#[async_trait]
pub trait TrustScoreRecalculationPort: Send + Sync {
    async fn request_recalculation(&self, party_id: Uuid) -> Result<(), ApplicationError>;
}

/// No-op implementation used until the real trust-score use case is wired in.
pub struct NoOpTrustScoreRecalculation;
#[async_trait]
impl TrustScoreRecalculationPort for NoOpTrustScoreRecalculation {
    async fn request_recalculation(&self, _party_id: Uuid) -> Result<(), ApplicationError> {
        Ok(())
    }
}
```

`NoOpTrustScoreRecalculation` is currently injected in `crates/api/src/main.rs` and in all test helpers. It must be replaced with a concrete `TrustScoreRecalculationService`.

### 6.2 Trigger points already wired

| Event | Use case | File | Action today |
|---|---|---|---|
| Review submitted | `SubmitReview` | `crates/application/src/reviews/submit_review.rs` | Calls `request_recalculation(reviewed_party_id)` |
| Dispute raised | `RaiseDispute` | `crates/application/src/disputes/raise_dispute.rs` | Increments `deals_disputed_count` for actor/against party and calls recalc |
| Dispute resolved | `ResolveDispute` | `crates/application/src/disputes/resolve_dispute.rs` | Calls recalc for both parties |
| Verification approved | `ApproveVerification` | `crates/application/src/verifications/approve_verification.rs` | Updates `verification_level` and calls recalc |
| Verification rejected | `RejectVerification` | `crates/application/src/verifications/reject_verification.rs` | Calls recalc |
| Verification revoked | `RevokeVerification` | `crates/application/src/verifications/revoke_verification.rs` | Calls recalc |

### 6.3 Trigger points still to wire

| Event | Use case | File | Action needed |
|---|---|---|---|
| Deal completed | `ExecuteTransition` | `crates/application/src/deals/execute_transition.rs` | Increment `deals_completed_count` for all parties, update `parties.total_deals_completed`, call recalc |
| Deal cancelled | `ExecuteTransition` | `crates/application/src/deals/execute_transition.rs` | Increment `deals_cancelled_count` for all parties, call recalc |
| Timeout-driven cancellation | `ProcessDealTimeouts` | `crates/application/src/deals/process_timeouts.rs` | Increment cancellation/timeout counters, call recalc |
| Profile updated | `UpdateParty` / `AddPartyRole` / `RemovePartyRole` | `crates/application/src/parties/` | Recompute `profile_completeness`, call recalc |
| Message sent/read | Message handlers | `crates/api/src/handlers/messages/` | Update response-time metrics (or trigger recalc which recomputes them) |
| Nightly decay job | Background job | TBD | Recompute time-based components |

### 6.4 Recommended concrete wiring

**Deal completion (`ExecuteTransition` → `Completed`)**

```rust
// Inside the COMPLETED branch, after deal_repo.update:
for party_id in deal.participating_party_ids() {
    trust_score_repo
        .increment_deals_completed_count(party_id, deal.total_deal_value)
        .await?;
    party_repo
        .increment_total_deals_completed(party_id)
        .await?;
    self.recalc.request_recalculation(party_id).await?;
}
```

**Deal cancellation (`ExecuteTransition` → `Cancelled`)**

```rust
for party_id in deal.participating_party_ids() {
    trust_score_repo
        .increment_deals_cancelled_count(party_id)
        .await?;
    self.recalc.request_recalculation(party_id).await?;
}
```

**Profile update**

```rust
let completeness = ProfileCompletenessCalculator::for_party(&party);
trust_score_repo
    .update_profile_completeness(party.id, completeness)
    .await?;
self.recalc.request_recalculation(party.id).await?;
```

---

## 7. Score Components & Weights

The overall score is a weighted average of eight components, each normalised to `0 – 100`.

```text
TRUST_SCORE = Σ (Component_i × Weight_i)
```

| # | Component | Symbol | Weight | Update trigger |
|---|---|---|---|---|
| 1 | Transaction History | `W_tx` | 0.25 | Deal completed / cancelled / disputed / timeout |
| 2 | Review Ratings | `W_rev` | 0.20 | Review submitted / edited / hidden |
| 3 | Profile Completeness | `W_prof` | 0.10 | Profile updated |
| 4 | Verification Level | `W_ver` | 0.15 | Verification approved / rejected / revoked |
| 5 | Response Rate | `W_resp` | 0.10 | Message sent / read (or nightly recalc) |
| 6 | Dispute History | `W_disp` | 0.10 | Dispute raised / resolved |
| 7 | Platform Longevity | `W_age` | 0.05 | Daily |
| 8 | Community Contribution | `W_comm` | 0.05 | Referral / recognition event |

The weights are persisted in `trust_scores.calculation_formula` so they can be tuned without code changes:

```json
{
  "weights": {
    "transaction_history": 0.25,
    "review_ratings": 0.20,
    "profile_completeness": 0.10,
    "verification_level": 0.15,
    "response_rate": 0.10,
    "dispute_history": 0.10,
    "longevity": 0.05,
    "community": 0.05
  },
  "version": "1.0.0",
  "computed_at": "2026-06-14T10:00:00Z"
}
```

---

## 8. Component Formulas

### 8.1 Transaction History Score (`TX_Score`, 0–100)

Based on deal outcomes where this Party participated.

```text
TX_Score = min(100, Base_TX + Volume_Bonus + Value_Bonus + Consistency_Bonus) - Penalties
```

**Base & bonuses:**

| Element | Formula | Cap |
|---|---|---|
| Base | `min(50, completed_deals × 10)` | 50 |
| Volume bonus | `min(20, max(0, completed_deals - 5) × 2)` | 20 |
| Value bonus | `min(15, total_completed_value / 100,000 × 15)` | 15 |
| Consistency bonus | `min(15, monthly_avg_completed_deals × 3)` | 15 |

**Penalties:**

| Event | Penalty |
|---|---|
| Deal cancelled after acceptance | `-5` per cancellation |
| No-show (accepted but did not participate) | `-10` per incident |
| Deal timeout / no response | `-2` per timeout |
| Disputed deal (regardless of outcome) | `-3` per dispute |

**Source of truth:** after the trust module is implemented, the calculator reads the counters from `trust_scores.deals_completed_count`, `trust_scores.deals_cancelled_count`, and `trust_scores.deals_disputed_count`. Until those counters are maintained incrementally, it can fall back to:

```sql
SELECT
  COUNT(*) FILTER (WHERE d.deal_status = 'COMPLETED')   AS completed,
  COUNT(*) FILTER (WHERE d.deal_status = 'CANCELLED')   AS cancelled,
  COUNT(*) FILTER (WHERE d.deal_status = 'DISPUTED')    AS disputed,
  COALESCE(SUM(d.total_deal_value) FILTER (WHERE d.deal_status = 'COMPLETED'), 0) AS completed_value
FROM deal_participations dp
JOIN deals d ON d.id = dp.deal_id
WHERE dp.party_id = $1;
```

**Timeout / no-show tracking:** the formula requires counts that do not exist today. Choose one of:

- **Option A (incremental):** add `timeouts_count INTEGER DEFAULT 0` and `no_shows_count INTEGER DEFAULT 0` to `trust_scores`. Increment them in `process_timeouts.rs` and no-show detection.
- **Option B (event-sourced):** derive counts from `deal_history` event types (`DEAL_TIMEOUT_TRANSITION`, `PARTY_NO_SHOW`, etc.) during recalculation.

> **Implementation note:** `deals_completed_count` and `deals_cancelled_count` are **not** updated by the current deal lifecycle code. This is the highest-priority prerequisite.

### 8.2 Review Ratings Score (`REV_Score`, 0–100)

```text
Review_Score_j = (communication + reliability + quality + timeliness) / 4
Weighted_Avg_Rating = Σ (Review_Score_j × Reviewer_Trust_j × Recency_j × Value_j)
                      / Σ (Reviewer_Trust_j × Recency_j × Value_j)
REV_Score = Weighted_Avg_Rating × 20
```

**Weighting factors:**

| Factor | Rule |
|---|---|
| `Reviewer_Trust_j` | `reviewer_party.overall_score / 100`. If the reviewer has no score yet, default to `0.5` (neutral). |
| `Recency_j` | ≤90 days = 1.0; 91–180 = 0.85; 181–365 = 0.70; 1–2 years = 0.50; >2 years = 0.30 |
| `Value_j` | `min(1.5, sqrt(deal_total_value / 10,000))`. If `deal_total_value` is missing, use `1.0`. |

**Cold-start rule:** until a Party has received at least 3 reviews, `REV_Score` starts at `50` (neutral) and is blended with the weighted average as reviews accumulate:

```text
if review_count == 0:      REV_Score = 50
elif review_count < 3:     REV_Score = 50 × (1 - review_count/3) + weighted_score × (review_count/3)
else:                      REV_Score = weighted_score
```

**Admin-hidden reviews:** when `HideReview` sets `is_public = false`, the review text is hidden from public view but the rating **still counts** toward the trust score. Hidden reviews should be flagged in `calculation_formula` for audit.

> **Implementation note:** Reviews are implemented; the recalculation that consumes them is not.

### 8.3 Profile Completeness Score (`PROF_Score`, 0–100)

Profile completeness is computed from fields that exist in the current schema. Fields such as a bio, portfolio, or external links are not present yet and should be added to the table when they are introduced.

| Section | Points | Rule |
|---|---|---|
| Basic info (display name, email, phone) | 20 | All three present = 20; email + display name = 15; email only = 10. |
| Location (`latitude`/`longitude`) | 15 | Both coordinates present = 15; one present = 8. |
| Business details (`tax_id`, `primary_domain_id`) | 15 | Each contributes 7.5 when present. |
| Role profile(s) filled | 40 | For each active role, compute a sub-score from the role profile JSON; average across roles. A non-default profile contributes at least 10 per role. |
| Service radius / preferences | 10 | `service_radius_km` present = 5; role profile contains preferences = 5. |
| **Maximum** | **100** | |

**Role profile sub-score (per role):**

```text
SupplierProfile:    resource_type_ids (3+) + typical_capacity + availability_schedule + preferred_compensation + insurance_verified
ConsumerProfile:    need_category_ids (3+) + typical_volume + preferred_quality_standard + budget_range + preferred_payment_terms
EnhancerProfile:    enhancement_type_ids (3+) + skills (3+) + certifications + hourly_rate/fixed_rate + equipment_owned + availability
```

Each populated field contributes an equal share of the 40 points for that role. For example, a `SupplierProfile` with 5 boolean/optional fields gets `40 / 5 = 8` points per populated field.

> **Implementation note:** `profile_completeness` is **not** computed today. Implement `ProfileCompletenessCalculator::for_party(&Party, &[PartyRole]) -> f64` and call it from party/role mutating use cases.

### 8.4 Verification Level Score (`VER_Score`, 0–100)

| Verification | Points |
|---|---|
| Email verified | 10 |
| Phone verified | 15 |
| Government ID verified | 30 |
| Business registration | 25 |
| Bank account linked | 10 |
| Professional certification | 10 |
| Video verification interview (bonus) | +10 |
| **Maximum** | **100** (110 with bonus, capped at 100) |

`trust_scores.verification_level` is an integer `0–5` summarising the highest verified tier. The mapping from approved verification points to levels is already implemented in the domain:

```rust
// crates/domain/src/entities/party_verification.rs
pub fn verification_level_from_points(points: i64) -> i32 {
    match points {
        0 => 0,
        10..=24 => 1,
        25..=54 => 2,
        55..=79 => 3,
        80..=99 => 4,
        _ => 5,
    }
}
```

| Level | Meaning |
|---|---|
| 0 | None |
| 1 | Email |
| 2 | Email + Phone |
| 3 | + Government ID |
| 4 | + Business registration |
| 5 | + Bank account + certification |

> **Implementation note:** `verification_level` **is** updated by `ApproveVerification` today.

### 8.5 Response Rate Score (`RESP_Score`, 0–100)

```text
Response_Rate = min(1.0, responses_sent_in_last_90_days / messages_received_in_last_90_days)
Timeliness_Score = lookup(average_response_time)
RESP_Score = Response_Rate × 80 + Timeliness_Score × 20
```

| Avg response time | Timeliness score |
|---|---|
| < 1 hour | 100 |
| < 4 hours | 80 |
| < 24 hours | 60 |
| < 48 hours | 40 |
| < 72 hours | 20 |
| ≥ 72 hours | 0 |

**Computing `average_response_hours`:**

For each party, over the last 90 days:

```sql
WITH response_times AS (
  SELECT
    m.recipient_party_id AS party_id,
    mr.read_at - m.created_at AS response_time
  FROM messages m
  JOIN message_reads mr ON mr.message_id = m.id
  WHERE m.created_at > now() - interval '90 days'
    AND m.recipient_party_id IS NOT NULL
)
SELECT party_id, EXTRACT(EPOCH FROM AVG(response_time)) / 3600.0 AS avg_response_hours
FROM response_times
GROUP BY party_id;
```

**Recommended approach:** compute this inside `RecalculateTrustScore` rather than incrementally, because it is naturally recomputed from the full message history. Store the result in `trust_scores.average_response_hours`.

**Performance note:** for high-volume message tables, consider materializing this in an hourly/daily aggregation table or using a covering index on `messages.created_at` and `messages.recipient_party_id`.

**Edge cases:**
- No messages received → `Response_Rate = 1.0`, `Timeliness_Score = 50` (neutral), `RESP_Score = 90`.
- No responses sent but messages received → `Response_Rate = 0.0`, `RESP_Score = Timeliness_Score × 20` (max 20).
- System messages and admin broadcasts should be excluded.

> **Implementation note:** Messages exist (`migrations/20260615145000_create_messages.sql`), but response-time tracking is **not** wired to the trust module.

### 8.6 Dispute History Score (`DISP_Score`, 0–100)

```text
DISP_Score = max(0, 100 - Dispute_Penalty)

Dispute_Penalty = (Disputes_Filed_Against × Severity_Factor)
                  + (Disputes_Lost × 15)
                  + Pattern_Penalty
```

| Resolution severity | Factor | Mapping from `disputes.resolution_type` |
|---|---|---|
| Resolved amicably | 5 | `AMICABLE` |
| Mediated resolution | 10 | `MEDIATED` |
| Lost arbitration | 20 | `ARBITRATED` where `resolution_outcome = 'IN_FAVOR_OF_RAISED'` against this party |

| Pattern | Penalty |
|---|---|
| 3+ disputes in 6 months | +10 |
| 5+ disputes in 6 months | +25 |

**`Disputes_Lost` rule:** a party "lost" a dispute when:
- They are `against_party_id` and `resolution_outcome = 'IN_FAVOR_OF_RAISED'`; or
- They are `raised_by_party_id` and `resolution_outcome = 'IN_FAVOR_OF_AGAINST'`; or
- `resolution_outcome = 'SPLIT'` counts as half a loss for both parties.

> **Implementation note:** `deals_disputed_count` **is** incremented by `RaiseDispute`, but the penalty formula is **not** applied until the recalculation module exists.

### 8.7 Platform Longevity Score (`AGE_Score`, 0–100)

```text
AGE_Score = min(100, account_age_days / 10)
```

Maximum at ~2.7 years (1,000 days). Inactivity decay: `-1` point per 30 days of inactivity after 90 days of no deal/message activity.

**Activity definition:** a party is "active" if, in the last 90 days, any of:
- They sent or received a deal-related message.
- They were a participant in a deal that changed status.
- They submitted or received a review.
- Their verification status changed.

> **Implementation note:** `longevity_days` is **not** computed today.

### 8.8 Community Contribution Score (`COMM_Score`, 0–100)

```text
COMM_Score = min(100,
  successful_referrals × 10
  + helpful_answers × 5
  + mentor_sessions × 15
  + community_recognition × 20
)
```

Community features are future capabilities; until then this component defaults to `0` and carries a small weight.

---

## 9. Role-Specific Scores

A Party that plays different roles in different deals gets a separate score per role.

```text
as_supplier_score  = weighted overall score using only supplier-role deals + supplier reviews
as_consumer_score  = weighted overall score using only consumer-role deals + consumer reviews
as_enhancer_score  = weighted overall score using only enhancer-role deals + enhancer reviews
```

For each role:

- `TX_Score` is recomputed from deals where `deal_participations.role = 'SUPPLIER'|'CONSUMER'|'ENHANCER'`.
- `REV_Score` is recomputed from reviews where `reviews.reviewed_role` matches the role.
- Other components (verification, profile, response, dispute, longevity, community) are shared across roles.

Role-specific scores are stored in:

```text
trust_scores.as_supplier_score
trust_scores.as_consumer_score
trust_scores.as_enhancer_score
```

and exposed in the API under `roleScores`.

**Implementation guidance:**

```sql
-- Completed supplier deals for a party
SELECT COUNT(*), COALESCE(SUM(d.total_deal_value), 0)
FROM deal_participations dp
JOIN deals d ON d.id = dp.deal_id
WHERE dp.party_id = $1
  AND dp.role = 'SUPPLIER'
  AND d.deal_status = 'COMPLETED';

-- Supplier reviews for a party
SELECT * FROM reviews
WHERE reviewed_party_id = $1 AND reviewed_role = 'SUPPLIER';
```

---

## 10. Trust Score Lifecycle

### 10.1 Events that trigger recalculation

| Event | Affected components | Action | Status |
|---|---|---|---|
| Deal completed | TX, REV, AGE | Recalculate; request reviews from all parties | Recalc and counters **not** wired |
| Deal cancelled | TX, DISP | Recalculate; apply cancellation penalty | Recalc and counters **not** wired |
| Deal disputed | TX, DISP | Recalculate; flag for moderation if pattern emerges | Implemented |
| Review submitted | REV | Recalculate weighted average | Implemented |
| Review hidden / challenged | REV, DISP | Recalculate if review is hidden or modified | Implemented |
| Profile updated | PROF | Recompute profile completeness | Not wired |
| Verification approved | VER, PROF | Recompute verification level | Implemented |
| Verification rejected / revoked | VER | Recompute verification level | Implemented |
| Message sent/read | RESP | Update response rate | Not wired |
| Dispute resolved | DISP | Apply penalties/rewards | Implemented |
| Nightly job | AGE, RESP, COMM, decay | Recalculate time-based components | Not implemented |

### 10.2 Recalculation flow

```text
Event occurs
    │
    ▼
┌─────────────────────────────┐
│ Load Party + trust_scores row│  ← upsert row if absent
└─────────────────────────────┘
    │
    ▼
┌─────────────────────────────┐
│ Fetch component inputs       │
│ (deals, reviews, messages,   │
│  verifications, disputes)    │
└─────────────────────────────┘
    │
    ▼
┌─────────────────────────────┐
│ Compute 8 components × roles │
└─────────────────────────────┘
    │
    ▼
┌─────────────────────────────┐
│ Weighted overall score       │
└─────────────────────────────┘
    │
    ▼
┌─────────────────────────────┐
│ Assign tier / badges         │
│ Update parties.trust_score   │
│ Update trust_scores row      │
│ Persist calculation_formula  │
└─────────────────────────────┘
    │
    ▼
┌─────────────────────────────┐
│ Tier changed?                │
│ → notify, update fee tier,   │
│   log audit                  │
└─────────────────────────────┘
```

### 10.3 Lazy row creation

Because the current code only creates `trust_scores` rows when a dispute is raised or a verification is approved, the recalculation use case must be idempotent and must create the row on first call if it does not exist:

```sql
INSERT INTO trust_scores (id, party_id, overall_score, ...)
VALUES (...)
ON CONFLICT (party_id) DO UPDATE SET ...;
```

### 10.4 End-to-end example: a deal completes

1. `VerifyMilestone` verifies the final milestone and creates a pending `ESCROW_RELEASE` transaction.
2. All three parties approve the transaction; `ApproveTransaction` releases escrow.
3. A party calls `ExecuteTransition(deal_id, Completed)`.
4. `ExecuteTransition` validates that all milestones are verified and all reviews are submitted.
5. `ExecuteTransition` updates `deals.deal_status = 'COMPLETED'` and `deals.actual_end_date`.
6. **New step:** for each `deal_participations.party_id`:
   - Increment `trust_scores.deals_completed_count`.
   - Either (a) add `total_completed_value` to `trust_scores` and accumulate `deals.total_deal_value`, or (b) compute the value bonus from `deals`/`deal_participations` on each recalc.
   - Increment `parties.total_deals_completed`.
   - Call `TrustScoreRecalculationPort::request_recalculation(party_id)`.
7. `TrustScoreRecalculationService` calls `RecalculateTrustScore::execute(party_id)`.
8. `RecalculateTrustScore` loads the party, trust-scores row, reviews, disputes, verifications, messages.
9. `TrustCalculator` computes the 8 components and overall score.
10. `RecalculateTrustScore` persists `trust_scores.overall_score`, role scores, `calculation_formula`, `last_calculated_at`.
11. `RecalculateTrustScore` updates `parties.trust_score = trust_scores.overall_score`.

---

## 11. Decay Rules

- **Inactivity decay:** after 180 days with no deal/message activity, the overall score decays by `2` points per month until activity resumes. Decay is capped: the score cannot drop below the verification-level floor.
- **Review recency:** older reviews automatically lose weight through the `Recency_j` factor.
- **Penalty expiry:** cancellation/no-show penalties remain for 12 months, then are halved; they fully expire after 24 months.

---

## 12. Transactions Feature

> **Implementation status:** The payments module is fully implemented. The summary below is preserved for reference; the live routes are defined in `crates/api/src/routes/payments.rs`.

### 12.1 Philosophy

Hayaland does **not** move real money. It maintains an internal **points ledger** denominated in `POINTS`. Every `Transaction` records a point movement that mirrors a physical settlement performed outside the platform.

Examples:

| Real-world event | Platform transaction |
|---|---|
| Consumer wires cash to Supplier | `ESCROW_RELEASE` or `DIRECT_TRANSFER` |
| Consumer deposits funds to platform wallet | `DEPOSIT` |
| Supplier withdraws earnings | `WITHDRAWAL` |
| Platform fee collected | `FEE` |
| Refund issued after cancellation | `ADJUSTMENT` |
| In-kind delivery recorded | `IN_KIND` |

### 12.2 Wallet model

Each Party has one `platform_wallets` row:

| Field | Meaning |
|---|---|
| `balance` | Points available for withdrawal or new deals |
| `escrow_balance` | Points held in escrow for active deals |
| `pending_balance` | Points awaiting transaction approval |
| `total_deposited` | Cumulative deposits |
| `total_withdrawn` | Cumulative withdrawals |

### 12.3 Transaction types

| Type | Description |
|---|---|
| `DEPOSIT` | Funds added to a Party's wallet |
| `WITHDRAWAL` | Funds removed from a Party's wallet |
| `ESCROW_HOLD` | Funds moved from wallet balance to escrow |
| `ESCROW_RELEASE` | Funds released from escrow to a recipient's wallet |
| `FEE` | Platform fee deducted |
| `ADJUSTMENT` | Refund, reversal, or manual correction |
| `IN_KIND` | Non-monetary value recorded for audit |

### 12.4 Transaction statuses

```text
PENDING → (all required parties approve) → VERIFIED → COMPLETE
     ↓ (any required party rejects)
REJECTED
```

| Status | Meaning |
|---|---|
| `PENDING` | Awaiting approvals |
| `VERIFIED` | All approvals received, ledger mutation applied |
| `COMPLETE` | Settlement fully reconciled (external confirmation if any) |
| `REJECTED` | At least one involved party rejected; no points moved |

### 12.5 Multi-party approval

Every transaction with `requires_approval = true` must be approved by every Party referenced in `from_party_id` and `to_party_id`.

Rules:

1. `approvals_required` is set to the number of distinct involved parties when the transaction is created.
2. Any active member of an involved Party may submit an approval on behalf of that Party.
3. A Party can only approve once per transaction (`UNIQUE (transaction_id, party_id)`).
4. Decision is either `APPROVED` or `REJECTED`.
5. If any involved Party rejects, the transaction moves to `REJECTED` and the ledger is not mutated.
6. When all involved Parties approve, the transaction moves to `VERIFIED` and the ledger mutation is applied atomically.

### 12.6 Ledger mutation rules

| Transaction type | From wallet | To wallet | Escrow impact |
|---|---|---|---|
| `DEPOSIT` | — | `to_party_id` | `balance += amount` |
| `WITHDRAWAL` | `from_party_id` | — | `balance -= amount` |
| `ESCROW_HOLD` | `from_party_id` | — | `balance -= amount`, `escrow_balance += amount` |
| `ESCROW_RELEASE` | — | `to_party_id` | `escrow_balance -= amount`, `to.balance += amount` |
| `FEE` | `from_party_id` | platform | `balance -= amount` or `escrow -= amount` |
| `ADJUSTMENT` | context-specific | context-specific | recorded with `description` |

All mutations are performed inside a single database transaction together with the final approval insert.

### 12.7 External settlement tracking

Because the platform does not hold fiat, each transaction records how the physical settlement was performed:

```text
payment_method: BANK_TRANSFER | CASH | CARD | CRYPTO | IN_KIND | OTHER
external_reference: "wire-ref-12345" | "receipt-abc" | null
```

These fields are for audit, dispute resolution, and future reconciliation with external payment providers.

---

## 13. Database Schema (Existing)

The following tables are already created by the existing migrations and are used as-is.

### 13.1 `trust_scores`

```sql
CREATE TABLE IF NOT EXISTS trust_scores (
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
```

### 13.2 `platform_wallets`

```sql
CREATE TABLE IF NOT EXISTS platform_wallets (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL UNIQUE REFERENCES parties(id) ON DELETE CASCADE,
    balance DECIMAL NOT NULL DEFAULT 0,
    escrow_balance DECIMAL NOT NULL DEFAULT 0,
    pending_balance DECIMAL NOT NULL DEFAULT 0,
    total_deposited DECIMAL NOT NULL DEFAULT 0,
    total_withdrawn DECIMAL NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'POINTS',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 13.3 `transactions`

```sql
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY,
    deal_id UUID REFERENCES deals(id),
    agreement_id UUID REFERENCES agreements(id),
    milestone_id UUID REFERENCES milestones(id),
    transaction_type TEXT NOT NULL,
    from_party_id UUID REFERENCES parties(id),
    to_party_id UUID REFERENCES parties(id),
    amount DECIMAL NOT NULL,
    currency TEXT NOT NULL DEFAULT 'POINTS',
    description TEXT,
    status TEXT NOT NULL DEFAULT 'PENDING',
    payment_method TEXT,
    external_reference TEXT,
    requires_approval BOOLEAN NOT NULL DEFAULT true,
    approvals_required INTEGER NOT NULL DEFAULT 2,
    approvals_received INTEGER NOT NULL DEFAULT 0,
    executed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 13.4 `transaction_approvals`

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

### 13.5 `agreements`

Created by the Agreement Generation & Signing feature. Relevant to transactions because `transactions.agreement_id` references the agreement under which a payment is made. `deal_id` is unique, so there is at most one agreement row per deal; renegotiation updates the same row and bumps `version`.

```sql
CREATE TABLE IF NOT EXISTS agreements (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL UNIQUE REFERENCES deals(id) ON DELETE CASCADE,
    agreement_status TEXT NOT NULL DEFAULT 'DRAFT'
        CHECK (agreement_status IN ('DRAFT','PENDING_SIGNATURES','SIGNED','EXECUTED','TERMINATED')),
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
```

### 13.6 `signatures`

Digital attestations recorded when a party signs the current agreement version. The unique key includes `version`, so a party can re-sign after a renegotiation produces a new agreement version.

```sql
CREATE TABLE IF NOT EXISTS signatures (
    id UUID PRIMARY KEY,
    agreement_id UUID NOT NULL REFERENCES agreements(id) ON DELETE CASCADE,
    party_id UUID NOT NULL REFERENCES parties(id),
    signed_by_user_id UUID NOT NULL REFERENCES users(id),
    signature_type TEXT NOT NULL,
    signature_data TEXT NOT NULL,
    ip_address TEXT,
    signed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    version INTEGER NOT NULL DEFAULT 1,
    UNIQUE (agreement_id, party_id, version)
);
```

---

## 14. Domain & Application Additions

### 14.1 Already implemented

- `crates/application/src/ports.rs` – `TrustScoreRecalculationPort` + `NoOpTrustScoreRecalculation`.
- `crates/application/src/reviews/`, `crates/application/src/disputes/`, `crates/application/src/verifications/` – trust-input use cases.
- `crates/application/src/payments/` – wallet and transaction use cases.
- `crates/api/src/routes/payments.rs` – live payment routes.

### 14.2 Still required for trust-score calculation

```text
crates/domain/src/entities/
  ├── trust_score.rs          # TrustScore, TrustTier, ScoreComponent

crates/domain/src/repositories/
  ├── trust_score_repository.rs

crates/domain/src/services/
  └── trust_calculator.rs     # Pure function: inputs -> TrustScore

crates/application/src/trust_scores/
  ├── get_trust_score.rs
  ├── recalculate_trust_score.rs
  └── dto.rs

crates/infrastructure/src/repositories/
  └── postgres_trust_score_repository.rs

crates/api/src/routes/
  └── trust_scores.rs

crates/api/src/handlers/trust_scores/
  ├── get_trust_score.rs
  └── recalculate_trust_score.rs
```

The concrete implementation of `TrustScoreRecalculationPort` should delegate to `RecalculateTrustScore`:

```rust
use application::trust_scores::RecalculateTrustScore;
use std::sync::Arc;
use uuid::Uuid;

pub struct TrustScoreRecalculationService {
    recalc: Arc<RecalculateTrustScore>,
}

impl TrustScoreRecalculationService {
    pub fn new(recalc: Arc<RecalculateTrustScore>) -> Self {
        Self { recalc }
    }
}

#[async_trait]
impl TrustScoreRecalculationPort for TrustScoreRecalculationService {
    async fn request_recalculation(&self, party_id: Uuid) -> Result<(), ApplicationError> {
        self.recalc.execute(party_id).await?;
        Ok(())
    }
}
```

---

## 15. API Contracts

### 15.1 Trust Score (not yet implemented)

> **Status:** These endpoints do not exist. They are the expected contract once the trust-score module is built.

#### Get trust score

```http
GET /api/v1/parties/{partyId}/trust
Authorization: Bearer <jwt>
```

Success `200`:

```json
{
  "trustScoreId": "ts-uuid-1",
  "partyId": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "partyDisplayName": "Green Acres Farm Ltd",
  "overallScore": 72,
  "scoreOutOf5": 3.60,
  "tier": "GOLD",
  "roleScores": {
    "asSupplier": {
      "score": 78,
      "dealsCompleted": 12,
      "dealsCancelled": 1,
      "averageRating": 4.7,
      "topStrengths": ["Reliable delivery", "Quality resources", "Good communication"]
    },
    "asConsumer": {
      "score": 65,
      "dealsCompleted": 3,
      "dealsCancelled": 0,
      "averageRating": 4.3,
      "topStrengths": ["Timely payment", "Clear requirements"]
    },
    "asEnhancer": {
      "score": null,
      "dealsCompleted": 0,
      "dealsCancelled": 0,
      "averageRating": null,
      "topStrengths": []
    }
  },
  "detailedMetrics": {
    "dealsCompletedCount": 15,
    "dealsCancelledCount": 1,
    "dealsDisputedCount": 0,
    "completionRate": 0.94,
    "averageResponseHours": 4.5,
    "profileCompleteness": 95.0,
    "verificationLevel": 4,
    "longevityDays": 380,
    "totalReviews": 18,
    "averageRating": 4.5
  },
  "componentBreakdown": {
    "transactionHistory": 80,
    "reviewRatings": 68,
    "profileCompleteness": 90,
    "verificationLevel": 75,
    "responseRate": 82,
    "disputeHistory": 95,
    "longevity": 50,
    "community": 30
  },
  "lastCalculatedAt": "2026-06-14T00:00:00Z",
  "nextCalculationAt": "2026-06-15T00:00:00Z",
  "calculationFormula": {
    "weights": { "transaction_history": 0.25, ... },
    "version": "1.0.0",
    "computedAt": "2026-06-14T00:00:00Z"
  }
}
```

#### Force recalculation (owner or admin)

```http
POST /api/v1/parties/{partyId}/trust/recalculate
Authorization: Bearer <jwt>
```

### 15.2 Wallets (implemented)

Live routes are defined in `crates/api/src/routes/payments.rs`:

```http
GET    /api/v1/parties/{id}/wallet
POST   /api/v1/parties/{id}/wallet/deposits
POST   /api/v1/parties/{id}/wallet/withdrawals
GET    /api/v1/parties/{id}/wallet/transactions
GET    /api/v1/parties/{party_id}/deals/{deal_id}/wallet
GET    /api/v1/parties/{party_id}/deals/{deal_id}/transactions
GET    /api/v1/payments/transactions/pending-approvals
GET    /api/v1/payments/transactions/{id}
POST   /api/v1/payments/transactions/{id}/approve
POST   /api/v1/payments/transactions/{id}/reject
```

---

## 16. Integration with the Deal Lifecycle

The current codebase implements the statuses `DRAFT → SUGGESTED → PENDING_REVIEW → NEGOTIATING → TERMS_LOCKED → COMMITTED → EXECUTING → COMPLETED`, plus `CANCELLED`, `DISPUTED`, `ON_HOLD`, `AWAITING_PARTY`, and `EXPIRED`. Trust and transaction actions are triggered mainly at the transitions below.

| Deal state | Agreement action | Trust action | Transaction action |
|---|---|---|---|
| `NEGOTIATING → TERMS_LOCKED` | Generate agreement; status becomes `PENDING_SIGNATURES` | — | — |
| `TERMS_LOCKED` | All parties sign → agreement status becomes `SIGNED` | — | — |
| `TERMS_LOCKED → COMMITTED` (requires `SIGNED` agreement) | — | — | Consumer `DEPOSIT` / `ESCROW_HOLD` for total deal value |
| `COMMITTED → EXECUTING` | — | — | Escrow confirmed, milestones enabled |
| Milestone verified | — | — | `ESCROW_RELEASE` transaction created in `PENDING`; supplier/enhancer approve |
| `EXECUTING → COMPLETED` | Mark agreement `EXECUTED` | Increment `deals_completed_count` for all parties; request reviews; recalc | Final releases completed |
| Deal cancelled | Mark agreement `TERMINATED` (admin-only) | Increment `deals_cancelled_count` for all parties; apply penalty; recalc | `ADJUSTMENT` refund transaction |
| Deal disputed | — | Increment `deals_disputed_count` for involved parties; recalc | Freeze related transactions until resolved |
| Review submitted | — | Recalculate review component | — |

> **Agreement pre-condition:** the existing `ExecuteTransition` use case enforces that the deal cannot move from `TERMS_LOCKED` to `COMMITTED` until the agreement is `SIGNED`. Trust/payment flows should therefore assume that any `COMMITTED` or later deal has a signed agreement, and should reference `transactions.agreement_id` when recording settlement transactions.

---

## 17. Worked Example

**Party:** Green Acres Farm Ltd  
**Historical deals:** 12 completed, 1 cancelled, 0 disputed  
**Total completed value:** 90,000 points  
**Reviews:** 18 reviews, weighted average rating 4.5  
**Profile completeness:** 95%  
**Verification level:** 4  
**Response rate:** 82.5% within 4 hours  
**Account age:** 380 days  
**Community:** 0 contributions  

**Component scores:**

| Component | Raw | Weight | Weighted |
|---|---|---|---|
| Transaction History | 80 | 0.25 | 20.0 |
| Review Ratings | 90 | 0.20 | 18.0 |
| Profile Completeness | 95 | 0.10 | 9.5 |
| Verification Level | 75 | 0.15 | 11.25 |
| Response Rate | 82 | 0.10 | 8.2 |
| Dispute History | 95 | 0.10 | 9.5 |
| Longevity | 38 | 0.05 | 1.9 |
| Community | 0 | 0.05 | 0.0 |
| **Overall** | — | **1.00** | **78.35** |

Result: **Gold tier**, overall score **78**, public score **3.90 / 5**.

---

## 18. Error Mapping

| Situation | Domain / Application error | HTTP status |
|---|---|---|
| Party has no trust score row | `PartyNotFound` / `NotFound` | 404 |
| Transaction not found | `TransactionNotFound` / `NotFound` | 404 |
| Wallet not found | `WalletNotFound` / `NotFound` | 404 |
| Caller is not a member of the acting party | `InsufficientPermissions` / `Forbidden` | 403 |
| Caller is not an involved party in the transaction | `InsufficientPermissions` / `Forbidden` | 403 |
| Transaction already resolved | `Validation` | 422 |
| Insufficient wallet balance | `Validation` | 422 |
| Rejected transaction cannot be approved | `Validation` | 422 |

---

## 19. Implementation Checklist

### Phase 1 — Wire trust-data producers

- [ ] Add repository helpers to increment `trust_scores.deals_completed_count` and update completed-value totals.
- [ ] Add repository helper to increment `trust_scores.deals_cancelled_count`.
- [ ] Update `ExecuteTransition` (`COMPLETED` branch) to increment counters and call recalc for all participating parties.
- [ ] Update `ExecuteTransition` (`CANCELLED` branch) to increment counters and call recalc for all participating parties.
- [ ] Update `ProcessDealTimeouts` to increment cancellation/timeout counters and call recalc on timeout-driven state changes.
- [ ] Decide storage for timeout/no-show penalties and implement population.
- [ ] Implement `ProfileCompletenessCalculator` and call it from `CreateParty`, `UpdateParty`, `AddPartyRole`, `RemovePartyRole`.
- [ ] Implement response-time computation (inside `RecalculateTrustScore` or message handlers).
- [ ] Ensure `trust_scores` row exists for every party (creation on party creation or upsert in recalculation).

### Phase 2 — Build the trust-score module

- [ ] Create `TrustScore` domain entity and `TrustTier` enum.
- [ ] Create `TrustScoreRepository` trait.
- [ ] Implement `TrustCalculator` pure domain service with all 8 components.
- [ ] Implement `PostgresTrustScoreRepository`.
- [ ] Implement `RecalculateTrustScore` use case.
- [ ] Implement `GetTrustScore` use case.
- [ ] Implement `TrustScoreRecalculationService` replacing `NoOpTrustScoreRecalculation`.
- [ ] Wire the new service in `crates/api/src/main.rs` and test helpers.
- [ ] Synchronise `trust_scores.overall_score` back to `parties.trust_score` after every recalculation.
- [ ] Add `GET /api/v1/parties/{id}/trust` and `POST /api/v1/parties/{id}/trust/recalculate` routes and handlers.
- [ ] Populate `trust_scores.calculation_formula` with weights and input snapshot on every recalculation.

### Phase 3 — Background jobs & polish

- [ ] Implement nightly decay/recalculation job.
- [ ] Compute `longevity_days` and apply inactivity decay.
- [ ] Implement role-specific scores (`as_supplier_score`, `as_consumer_score`, `as_enhancer_score`).
- [ ] Update Win-Win-Win validator to load real `parties.trust_score`.

### Phase 4 — Deferrable

- [ ] Matching engine.
- [ ] PartyGroup governance and group-level trust.
- [ ] Automated email/phone verification flows.
- [ ] Community contribution inputs and score.
- [ ] Trust-tier-based escrow percentages.
- [ ] Score history / time-series API.

---

## 20. Testing Strategy

### Unit tests

- `TrustCalculator` — test each component in isolation with known inputs:
  - Transaction history with various completed/cancelled/disputed counts.
  - Review weighted average with reviewer trust, recency, and value weights.
  - Profile completeness for parties with different field coverage.
  - Dispute penalties with different severities and outcomes.
  - Cold-start blending for parties with 0–3 reviews.

### Integration tests

- End-to-end: complete a deal → verify `trust_scores.deals_completed_count` increments → verify `parties.trust_score` is updated.
- Submit a review → verify recalculation is triggered and overall score changes.
- Raise a dispute → verify `deals_disputed_count` increments and dispute penalty affects score.
- Approve a verification → verify `verification_level` and score update.

### Regression tests

- Ensure `NoOpTrustScoreRecalculation` tests still pass by injecting the no-op in tests that do not exercise trust.
- Ensure search/list endpoints return non-zero `trust_score` after recalculation.

---

## 21. Open Points & Future Extensions

- **Market-benchmark provider:** integrate external pricing data to weight value-bonus more accurately.
- **Machine-learning risk score:** add a predicted default-risk component with human-review safeguards.
- **Graph trust network:** score parties based on the trustworthiness of their repeated partners.
- **Real payment provider bridge:** keep the `payment_method`/`external_reference` fields but optionally settle via Stripe/Flutterwave instead of purely mirroring external settlement.
- **Smart-contract settlement:** for blockchain-ready domains, emit settlement instructions from approved transactions.
- **Party-group governance:** group-level trust score derived from member scores and group deal history.

---

## 22. Glossary

| Term | Meaning |
|---|---|
| **Trust Score** | Composite reputation metric per party, 0–100, derived from deals, reviews, verification, and behaviour. |
| **Role Score** | Trust score scoped to a single deal role: Supplier, Consumer, or Enhancer. |
| **Points** | Platform-internal unit of value; transactions mirror external physical settlements. |
| **Platform Wallet** | A Party's ledger record: available balance, escrow balance, and pending balance. |
| **Transaction Approval** | Approval by every party involved in a transaction before it is marked verified/complete. |
| **Trust Tier** | Bronze / Silver / Gold / Platinum classification based on overall score. |
| **Agreement** | Legal text generated when a deal reaches `TERMS_LOCKED`, signed digitally by all parties before commitment. |
| **Signature** | A party's SHA-256 attestation recorded against an agreement version. |
| **Decay** | Scheduled reduction of score for inactivity or ageing penalties. |
