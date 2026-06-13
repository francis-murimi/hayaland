# Negotiation Guide — Hayaland 3-Party Deal Platform

> **Scope:** This document describes the negotiation phase of a 3-Party Deal: how parties propose and agree on **terms**, how **value** is distributed among Supplier, Consumer, Enhancer, and the platform, and how the **Win-Win-Win validator** decides whether a deal may proceed to commitment.
>
> **Audience:** Backend engineers, API consumers, and product owners building on the Hayaland deal lifecycle.
>
> **Based on:** `hayaland-deal-plan.md`, `party-guide.md`, `3partydeal.pdf`, and the current migration/schema design.

---

## 1. What This Guide Covers

A Hayaland deal is not committed the moment it is drafted. The parties must:

1. Negotiate the **terms** (price, timeline, deliverables, quality standards, risk allocation, etc.).
2. Agree on a **value distribution** that splits the total deal value among Supplier, Consumer, Enhancer, and the platform.
3. Pass the **Win-Win-Win validator**, which guarantees that all three parties gain positive net value and that the split is sufficiently fair.

This guide explains the domain model, rules, API contracts, and worked examples for those three capabilities.

---

## 2. When Negotiation Happens

Negotiation is only possible while the deal is in an open, non-terminal state. The primary negotiation window is:

```
DRAFT → SUGGESTED → PENDING_REVIEW → NEGOTIATING → TERMS_LOCKED → COMMITTED
```

| State | Negotiation allowed? | Notes |
|---|---|---|
| `DRAFT` | Yes, by initiator only | Terms and value distribution can be pre-filled before submission. |
| `SUGGESTED` | Yes | Other parties review the draft and may start proposing changes. |
| `PENDING_REVIEW` | Yes | Formal review window before active negotiation. |
| `NEGOTIATING` | Yes | Main negotiation state. Parties propose, counter, accept, and reject terms. |
| `TERMS_LOCKED` | No | Terms are frozen. Returning to negotiation requires an explicit transition. |
| `COMMITTED`+ | No | Deal is locked in. Changes require cancellation or dispute resolution. |
| `ON_HOLD` / `AWAITING_PARTY` | Limited | Parties can return to `NEGOTIATING` to resolve blockers. |
| `COMPLETED` / `CANCELLED` / `EXPIRED` | No | Terminal states. |

A deal moves from `NEGOTIATING` to `TERMS_LOCKED` when **all mandatory terms are accepted** and a **value distribution** is present.

---

## 3. Terms

A `Term` is a single negotiable clause within a deal. Terms are versioned and immutable once accepted.

### 3.1 Term entity

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Primary key. |
| `deal_id` | UUID | Parent deal. |
| `proposed_by_party_id` | UUID | Party that created this version. |
| `term_type` | Text | e.g. `PRICE`, `DELIVERY_DATE`, `QUALITY_STANDARD`, `PAYMENT_TERMS`, `LIABILITY_CAP`. |
| `term_name` | Text | Human-readable label. |
| `description` | Text | Detailed clause text (Markdown supported by convention). |
| `negotiation_status` | Enum | `PROPOSED`, `ACCEPTED`, `REJECTED`, `COUNTERED`, `WITHDRAWN`. |
| `parent_term_id` | UUID | Previous version of this term, forming a lineage. |
| `version` | Integer | Starts at 1; incremented on every counter. |
| `is_mandatory` | Boolean | Must be accepted before `TERMS_LOCKED`. |
| `proposed_at` | Timestamp | When this version was created. |
| `resolved_at` | Timestamp | When the term reached a terminal status. |
| `resolution` | Text | Optional note explaining the resolution. |

### 3.2 Term statuses

```
PROPOSED → ACCEPTED
        → REJECTED
        → COUNTERED  (creates a new PROPOSED version)
        → WITHDRAWN
```

- `PROPOSED` — active offer on the table.
- `ACCEPTED` — frozen; no further edits allowed.
- `REJECTED` — terminal; a new version must be proposed to restart discussion.
- `COUNTERED` — the proposing party accepts that this version is superseded by a counter-proposal.
- `WITHDRAWN` — proposer pulled the offer before anyone responded.

### 3.3 Rules

1. **Immutability of accepted terms.** An `ACCEPTED` term cannot be edited or re-countered. To change it, a party must move the deal back to `NEGOTIATING` and propose a new version.
2. **Version lineage.** Every counter creates a new `Term` row with `parent_term_id` pointing to the previous version. The UI/audit log can walk this chain to show negotiation history.
3. **Mandatory terms.** Any term with `is_mandatory = true` must be `ACCEPTED` before `TERMS_LOCKED` is allowed.
4. **Who can act.** Only active members of a participating party may propose, counter, accept, reject, or withdraw terms on behalf of that party.
5. **Single-thread MVP.** In the first implementation, terms are negotiated one clause at a time (no dimension locking). Future versions may support parallel threads on independent dimensions.

### 3.4 Lifecycle actions

| Action | HTTP | Effect |
|---|---|---|
| Propose a term | `POST /deals/{id}/terms` | Creates a `PROPOSED` term (version 1). |
| Counter a term | `POST /deals/{id}/terms/{termId}/counter` | Marks current term `COUNTERED`; creates new `PROPOSED` version. |
| Accept a term | `POST /deals/{id}/terms/{termId}/accept` | Marks term `ACCEPTED`, sets `resolved_at`. |
| Reject a term | `POST /deals/{id}/terms/{termId}/reject` | Marks term `REJECTED`; requires a new proposal to continue. |
| Withdraw a term | `POST /deals/{id}/terms/{termId}/withdraw` | Marks term `WITHDRAWN`; only the proposing party can do this. |

---

## 4. Value Distribution

All value in the MVP is expressed in **platform points** (`POINTS`). Each deal has its own value distribution that records who receives what and when.

### 4.1 Value distribution entity

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Primary key. |
| `deal_id` | UUID | Parent deal (one-to-one). |
| `total_value` | Decimal | Total deal value in points. |
| `currency` | Text | Always `POINTS` in the MVP. |
| `distribution_model` | Enum | `FIXED_PRICE`, `REVENUE_SHARE`, `COST_PLUS`, `BARTER`, `HYBRID`. |
| `supplier_share_percentage` | Decimal | Share of `total_value` the Supplier receives. |
| `supplier_share_amount` | Decimal | Supplier amount in points. |
| `consumer_cost_percentage` | Decimal | Share of `total_value` the Consumer must provide (usually 100%). |
| `consumer_cost_amount` | Decimal | Consumer cost in points. |
| `enhancer_share_percentage` | Decimal | Share of `total_value` the Enhancer receives. |
| `enhancer_share_amount` | Decimal | Enhancer amount in points. |
| `platform_fee_percentage` | Decimal | Per-deal platform fee percentage. |
| `platform_fee_amount` | Decimal | Platform fee in points. |
| `payment_schedule` | JSONB | Array of scheduled payments/milestones. |
| `win_win_win_score` | Decimal | Last computed validation score. |

### 4.2 Invariants

A valid `ValueDistribution` must satisfy:

1. `supplier_share_amount + enhancer_share_amount + platform_fee_amount = total_value`.
2. `consumer_cost_amount ≤ total_value` in pure consumer-funded models; may differ for `BARTER` or `HYBRID`.
3. `supplier_share_percentage + enhancer_share_percentage + platform_fee_percentage = 100` (within `0.0001` tolerance).
4. Each party receives **at least 5%** of `total_value`.
5. `platform_fee_percentage` must be within the platform's configured min/max bounds (default configurable per category).

### 4.3 Payment schedule

`payment_schedule` is a JSONB array of entries:

```json
[
  {
    "sequence": 1,
    "trigger": "UPFRONT",
    "due_at": "2026-08-01",
    "amount": 5000,
    "recipient_role": "SUPPLIER",
    "milestone_id": null
  },
  {
    "sequence": 2,
    "trigger": "MILESTONE",
    "due_at": null,
    "amount": 15000,
    "recipient_role": "ENHANCER",
    "milestone_id": "<uuid>"
  }
]
```

Allowed triggers: `UPFRONT`, `MILESTONE`, `ON_DELIVERY`, `DEFERRED`.

### 4.4 Sync with the `Deal` aggregate

When a value distribution is set or updated, the application layer copies the totals into the `Deal` aggregate:

- `deals.total_deal_value = total_value`
- `deals.platform_fee_percentage = platform_fee_percentage`
- `deals.platform_fee_amount = platform_fee_amount`

This keeps the deal summary queryable without joining `value_distributions` every time.

---

## 5. Win-Win-Win Validator

The validator is a pure domain service (`WinWinWinValidator`) with no repository dependencies. It takes a snapshot of the deal, terms, value distribution, and parties, and returns a structured validation result.

### 5.1 Purpose

Ensure the deal is **economically sound** and **fair** before commitment:

- Every party must gain more than they give up.
- No party should capture a disproportionate share of the surplus.
- The consumer must not pay more than reasonable compared to independent sourcing.
- The deal must meet a minimum size threshold.

### 5.2 Inputs

| Input | Source |
|---|---|
| `ValueDistribution` | Deal child entity. |
| `Resource`, `Need`, `Enhancement` | Deal child entities (cost/opportunity estimates). |
| Party snapshots | Trust score, verification level, active-deal counts. |
| Domain config | Min deal size, max share thresholds, discount rate. |
| Market benchmarks | Optional; from a `MarketBenchmarkProvider` port. |

### 5.3 Critical rules (block commitment)

A deal with any critical violation receives status `Blocked` and cannot proceed past `TERMS_LOCKED`.

| Rule | Violation example |
|---|---|
| All party gains > 0 | Supplier receives 0 points; deal gives them no net benefit. |
| No single share > 70% | Supplier captures 80% of total value. |
| Consumer cost ≤ independent sourcing × 1.05 | Consumer pays 10,000 points when market price is 8,000. |
| Deal value ≥ min deal size | Total value is below the category minimum (e.g. 500 points). |
| All mandatory terms accepted | A required liability clause is still `PROPOSED`. |

### 5.4 Warning rules (require explicit acknowledgment)

Warnings do not block commitment, but the UI must show them and record that parties acknowledged them.

| Rule | Warning example |
|---|---|
| Any party share < 10% | Enhancer only gets 3% of total value. |
| Enhancer fee/value-added outside 50–150% of reference | Enhancer compensation is far from typical market range. |
| Risk ratio between max/min party > 3.0 | One party has 3× more at stake than another. |
| Unbalanced payment schedule | More than 80% of value is released before any milestone verification. |

### 5.5 Fairness score

The validator computes a score from 0 to 100 using a weighted formula:

| Component | Weight | Description |
|---|---|---|
| Absolute gain | 25% | Average net gain across all three parties. |
| Proportional fairness | 30% | Penalty based on variance of normalized gains (Gini-inspired). |
| Market benchmark | 25% | How the deal compares to independent sourcing or market rates. |
| Opportunity cost | 20% | Whether each party's gain exceeds their next-best alternative. |

#### Status buckets

| Score range | Status | Meaning |
|---|---|---|
| 90–100 | `Excellent` | Strong Win-Win-Win; no action required. |
| 70–89 | `Good` | Meets commitment threshold. |
| 50–69 | `Fair` | Advisory only at draft time; must improve to `Good` before lock. |
| 1–49 | `Poor` | Likely blocked unless terms are renegotiated. |
| 0 | `Blocked` | Critical violation present. |

### 5.6 Integration gates

| Use case / transition | Validator behavior |
|---|---|
| `CreateDraftDeal` / `UpdateDraftDeal` | Advisory; result shown as feedback but does not block saving. |
| `SetValueDistribution` | Advisory; updates `win_win_win_score` on the distribution row. |
| `SubmitDeal` (Draft → Suggested) | Requires `Good` or better and no critical violations. |
| `LockTerms` (Negotiating → TermsLocked) | Requires `Good` or better and no critical violations. |
| `Commit` (TermsLocked → Committed) | Hard gate; re-runs validation and blocks on any critical or warning that has not been acknowledged. |

### 5.7 Validation result structure

```json
{
  "score": 82,
  "status": "Good",
  "blocked": false,
  "violations": [],
  "warnings": [
    {
      "code": "ENHANCER_SHARE_LOW",
      "message": "Enhancer share is 8%, below the 10% guidance.",
      "party_role": "ENHANCER"
    }
  ],
  "party_feedback": {
    "SUPPLIER": { "net_gain": 12000, "roi_percent": 24 },
    "CONSUMER": { "net_gain": 5000, "roi_percent": 10 },
    "ENHANCER": { "net_gain": 3000, "roi_percent": 15 }
  }
}
```

---

## 6. API Contracts

All endpoints require an `Authorization: Bearer <jwt>` header. Deal endpoints also require `X-Party-ID` when the user belongs to more than one party.

### 6.1 Terms

#### List terms
```http
GET /api/v1/deals/{deal_id}/terms
```

Response:
```json
{
  "terms": [
    {
      "id": "...",
      "term_type": "PRICE",
      "term_name": "Crop purchase price",
      "description": "Consumer will pay 18,000 points per hectare of delivered produce.",
      "negotiation_status": "ACCEPTED",
      "version": 1,
      "is_mandatory": true,
      "proposed_by_party_id": "..."
    }
  ]
}
```

#### Propose a term
```http
POST /api/v1/deals/{deal_id}/terms
Content-Type: application/json

{
  "term_type": "DELIVERY_DATE",
  "term_name": "Delivery deadline",
  "description": "All produce must be delivered by 2026-09-30.",
  "is_mandatory": true
}
```

Response: `201 Created` with the created term.

#### Counter a term
```http
POST /api/v1/deals/{deal_id}/terms/{term_id}/counter
Content-Type: application/json

{
  "description": "All produce must be delivered by 2026-10-15.",
  "is_mandatory": true
}
```

Response: `201 Created` with the new version.

#### Accept / reject / withdraw
```http
POST /api/v1/deals/{deal_id}/terms/{term_id}/accept
POST /api/v1/deals/{deal_id}/terms/{term_id}/reject
POST /api/v1/deals/{deal_id}/terms/{term_id}/withdraw
```

All return the updated term.

### 6.2 Value distribution

#### Get distribution
```http
GET /api/v1/deals/{deal_id}/value-distribution
```

Response:
```json
{
  "total_value": 30000,
  "currency": "POINTS",
  "distribution_model": "FIXED_PRICE",
  "supplier_share_percentage": 60,
  "supplier_share_amount": 18000,
  "consumer_cost_percentage": 100,
  "consumer_cost_amount": 30000,
  "enhancer_share_percentage": 30,
  "enhancer_share_amount": 9000,
  "platform_fee_percentage": 10,
  "platform_fee_amount": 3000,
  "payment_schedule": [
    { "sequence": 1, "trigger": "UPFRONT", "amount": 9000, "recipient_role": "SUPPLIER" },
    { "sequence": 2, "trigger": "ON_DELIVERY", "amount": 9000, "recipient_role": "SUPPLIER" },
    { "sequence": 3, "trigger": "ON_DELIVERY", "amount": 9000, "recipient_role": "ENHANCER" }
  ],
  "win_win_win_score": 82
}
```

Note: in this example the platform fee is included in the 100% split, so `supplier_share_amount + enhancer_share_amount + platform_fee_amount = 30000`.

#### Set distribution
```http
PUT /api/v1/deals/{deal_id}/value-distribution
Content-Type: application/json

{
  "total_value": 30000,
  "distribution_model": "FIXED_PRICE",
  "supplier_share_percentage": 60,
  "enhancer_share_percentage": 30,
  "platform_fee_percentage": 10,
  "payment_schedule": [
    { "sequence": 1, "trigger": "UPFRONT", "amount": 9000, "recipient_role": "SUPPLIER" },
    { "sequence": 2, "trigger": "ON_DELIVERY", "amount": 9000, "recipient_role": "SUPPLIER" },
    { "sequence": 3, "trigger": "ON_DELIVERY", "amount": 9000, "recipient_role": "ENHANCER" }
  ]
}
```

The server computes `consumer_cost_amount`, `supplier_share_amount`, etc., and runs the Win-Win-Win validator in advisory mode.

### 6.3 Validation

#### Run validation explicitly
```http
POST /api/v1/deals/{deal_id}/validate
```

Response: the validation result JSON shown in §5.7.

---

## 7. State-Transition Gates

| Transition | Required preconditions |
|---|---|
| `DRAFT → SUGGESTED` | Three distinct participations; all mandatory terms accepted or none yet proposed; value distribution present; validation `Good`+. |
| `SUGGESTED → PENDING_REVIEW` | Any party accepts participation; validation still `Good`+. |
| `PENDING_REVIEW → NEGOTIATING` | All three parties have accepted participation. |
| `NEGOTIATING → TERMS_LOCKED` | All mandatory terms `ACCEPTED`; value distribution set; validation `Good`+ with no critical violations. |
| `TERMS_LOCKED → COMMITTED` | Agreement generated and signed by all parties; escrow funded; validation re-run and passing with acknowledgments for any warnings. |
| `TERMS_LOCKED → NEGOTIATING` | Explicit renegotiation by any party (resets `TERMS_LOCKED`). |
| `ON_HOLD → NEGOTIATING` | Blocker resolved; re-validation required before next lock. |

---

## 8. Worked Examples

### 8.1 Good deal

- **Resource:** 10 hectares of idle farmland.
- **Need:** 500 kg of organic tomatoes.
- **Enhancement:** Agrodealer provides seeds, fertilizer, and agronomy support.
- **Total value:** 30,000 points.
- **Distribution:** Supplier 60% (18,000), Enhancer 30% (9,000), Platform 10% (3,000).
- **Terms:** delivery date, quality standard, payment schedule all accepted.
- **Validator:** score 82, status `Good`, no critical violations.

Result: deal proceeds to `TERMS_LOCKED` and then `COMMITTED`.

### 8.2 Blocked deal

- **Total value:** 10,000 points.
- **Distribution:** Supplier 85% (8,500), Enhancer 5% (500), Platform 10% (1,000).

Validator output:

```json
{
  "score": 0,
  "status": "Blocked",
  "blocked": true,
  "violations": [
    {
      "code": "SHARE_EXCEEDS_MAX",
      "message": "Supplier share of 85% exceeds the 70% maximum.",
      "party_role": "SUPPLIER"
    },
    {
      "code": "PARTY_SHARE_TOO_LOW",
      "message": "Enhancer share of 5% is below the 10% minimum.",
      "party_role": "ENHANCER"
    }
  ],
  "warnings": [],
  "party_feedback": { ... }
}
```

Result: `LockTerms` and `Commit` are rejected with `WinWinWinValidationFailed`.

### 8.3 Renegotiation

1. Deal is `TERMS_LOCKED`.
2. Consumer notices a missing quality clause and requests renegotiation.
3. Transition `TERMS_LOCKED → NEGOTIATING` is executed.
4. Consumer proposes a new mandatory `QUALITY_STANDARD` term.
5. Supplier counters; Consumer accepts.
6. Value distribution is updated to reflect slightly higher Enhancer fee.
7. Validation re-run: score 78, status `Good`.
8. Deal returns to `TERMS_LOCKED`.

---

## 9. Error Mapping

Common errors raised during negotiation:

| Situation | `DomainError` / `ApplicationError` | HTTP status |
|---|---|---|
| Deal does not exist | `DealNotFound` | 404 |
| Term does not exist | `TermNotFound` | 404 |
| Actor is not a deal participant | `Forbidden` / `DealAccessDenied` | 403 |
| Deal is not in a negotiable state | `InvalidStateTransition` | 409 |
| Value distribution percentages do not sum to 100 | `InvalidValueDistribution` | 422 |
| Win-Win-Win validation fails | `WinWinWinValidationFailed` | 422 |
| Mandatory term not accepted before lock | `Validation` | 422 |

---

## 10. Open Points & Future Extensions

- **Multi-thread negotiation:** Allow independent dimensions (price, timeline, liability) to be negotiated in parallel with per-dimension locking.
- **ZOPA visualization:** Show each party the Zone Of Possible Agreement based on opportunity-cost estimates.
- **Market-benchmark provider:** Replace stub benchmarks with a real data source or oracle.
- **Legally binding signatures:** Move from SHA-256 attestation hashes to qualified e-signatures.
- **AI-assisted negotiation suggestions:** Recommend counters based on historical similar deals and trust scores.

---

## 11. Glossary

| Term | Meaning |
|---|---|
| **Term** | A negotiable clause in a deal, with versioning and status. |
| **Value Distribution** | Allocation of the total deal value to Supplier, Consumer, Enhancer, and platform. |
| **Win-Win-Win Validation** | Domain service that checks economic soundness and fairness before commitment. |
| **Payment Schedule** | JSONB array describing when and under what conditions value moves. |
| **Mandatory Term** | A term that must be `ACCEPTED` before `TERMS_LOCKED`. |
