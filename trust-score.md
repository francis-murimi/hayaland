# Trust Score & Transactions — Hayaland 3-Party Deal Platform

> **Scope:** This document specifies how Hayaland computes a **Trust Score** for every Party from historical deal outcomes, reviews, and platform behaviour, and how the internal **Transactions** ledger records point movements tied to those deals.
>
> **Audience:** Backend engineers, product owners, and API consumers.
>
> **Based on:** `3partydeal.pdf` Software Design Document (§3.21, §6.10, §8.4), `hayaland-deal-plan.md`, `deal-plan.md`, `party-guide.md`, `negotiation-guide.md`, and the existing `hayaland` Rust codebase.

---

## 1. Goals

1. **Reputation is data-driven.** Every Party's trust score is derived from observable facts: completed/cancelled/disputed deals, multi-dimensional reviews, verification level, response speed, profile completeness, and platform longevity.
2. **Role-aware scoring.** A Party can be a Supplier in one deal and a Consumer in another; trust is tracked per role (`as_supplier_score`, `as_consumer_score`, `as_enhancer_score`) as well as overall.
3. **Transparent, auditable formula.** The calculation weights and inputs are stored in `trust_scores.calculation_formula` so the score can be explained and replayed.
4. **Point ledger mirrors real settlement.** All value exchange on the platform is recorded in platform points. A `Transaction` represents a point movement that mirrors a physical settlement performed outside the platform (bank transfer, cash, in-kind delivery, etc.).
5. **Multi-party approval.** Every transaction that moves points between Parties requires approval from every involved Party before it becomes `VERIFIED`/`COMPLETE`.

---

## 2. Trust Score at a Glance

| Property | Value |
|---|---|
| Internal scale | `0 – 100` points |
| Public display scale | `0.00 – 5.00` stars (optional API field) |
| Tiers | Bronze `0–39`, Silver `40–59`, Gold `60–74`, Platinum `75–100` |
| Update frequency | Event-driven (deal completion, review submission, verification change, profile update) plus a nightly decay/recalculation job |
| Storage | `trust_scores` table, one row per Party |

A Party's trust score affects:

- **Matching priority** — higher scores are ranked earlier in `GET /api/v1/matches`.
- **Win-Win-Win validation** — high-trust parties reduce the opportunity-cost penalty in the validator.
- **Platform fee tier** — trusted Partners/Champions receive lower per-deal platform fees (future fee-policy feature).
- **Deal eligibility** — parties with too many disputes/cancellations may be blocked from initiating deals.

---

## 3. Data Sources

The trust engine reads from the following existing tables:

| Source table | Fields used |
|---|---|
| `parties` | `created_at`, `verification_status`, `location`, `primary_domain_id` |
| `party_roles` | `role_type`, `profile` (used for profile-completeness heuristics) |
| `deal_participations` | `party_id`, `role`, `participation_status`, `is_initiator` |
| `deals` | `deal_status`, `total_deal_value`, `deal_status = COMPLETED / CANCELLED / DISPUTED`, `actual_end_date` |
| `reviews` | `reviewed_party_id`, `reviewed_role`, `overall_rating`, `communication_rating`, `reliability_rating`, `quality_rating`, `timeliness_rating`, `created_at`, `is_public` |
| `party_verifications` | `verification_level`, `status = VERIFIED` |
| `messages` / `deal_history` | Response timestamps used to compute `average_response_hours` |
| `disputes` | `against_party_id`, `dispute_status`, `resolution_type` |

> **Note:** `party_verifications` and `messages` are defined in the design documents but are not yet implemented in the current migrations. The trust calculator must treat them as optional inputs and fall back to defaults when missing.

---

## 4. Score Components & Weights

The overall score is a weighted average of eight components, each normalised to `0 – 100`.

```text
TRUST_SCORE = Σ (Component_i × Weight_i)
```

| # | Component | Symbol | Weight | Update trigger |
|---|---|---|---|---|
| 1 | Transaction History | `W_tx` | 0.25 | Deal completed / cancelled / disputed |
| 2 | Review Ratings | `W_rev` | 0.20 | Review submitted / edited / challenged |
| 3 | Profile Completeness | `W_prof` | 0.10 | Profile updated |
| 4 | Verification Level | `W_ver` | 0.15 | Verification approved |
| 5 | Response Rate | `W_resp` | 0.10 | Message sent / read |
| 6 | Dispute History | `W_disp` | 0.10 | Dispute resolved |
| 7 | Platform Longevity | `W_age` | 0.05 | Daily |
| 8 | Community Contribution | `W_comm` | 0.05 | Referral / recognition event |

The weights above are persisted in `trust_scores.calculation_formula` so they can be tuned without code changes:

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

## 5. Component Formulas

### 5.1 Transaction History Score (`TX_Score`, 0–100)

Based entirely on deal outcomes where this Party participated.

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

The source of truth for completed/cancelled/disputed counts is:

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

### 5.2 Review Ratings Score (`REV_Score`, 0–100)

```text
Review_Score_j = (communication + reliability + quality + timeliness) / 4
Weighted_Avg_Rating = Σ (Review_Score_j × Reviewer_Trust_j × Recency_j × Value_j)
                      / Σ (Reviewer_Trust_j × Recency_j × Value_j)
REV_Score = Weighted_Avg_Rating × 20
```

**Weighting factors:**

| Factor | Rule |
|---|---|
| `Reviewer_Trust_j` | `reviewer_party.overall_score / 100` |
| `Recency_j` | ≤90 days = 1.0; 91–180 = 0.85; 181–365 = 0.70; 1–2 years = 0.50; >2 years = 0.30 |
| `Value_j` | `min(1.5, sqrt(deal_total_value / 10,000))` |

**Cold-start rule:** until a Party has received at least 3 reviews, `REV_Score` starts at `50` (neutral) and is blended with the weighted average as reviews accumulate.

### 5.3 Profile Completeness Score (`PROF_Score`, 0–100)

| Section | Points |
|---|---|
| Basic info (display name, email, phone) | 20 |
| Bio / description | 15 |
| Location verified (location_geo present + address) | 10 |
| Business details (tax_id, primary_domain_id) | 15 |
| Portfolio / past work | 15 |
| Skills / specializations (from role profile) | 10 |
| Availability / preferences | 10 |
| External links | 5 |
| **Maximum** | **100** |

### 5.4 Verification Level Score (`VER_Score`, 0–100)

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

`trust_scores.verification_level` is an integer `0–5` summarising the highest verified tier:

| Level | Meaning |
|---|---|
| 0 | None |
| 1 | Email |
| 2 | Email + Phone |
| 3 | + Government ID |
| 4 | + Business registration |
| 5 | + Bank account + certification |

### 5.5 Response Rate Score (`RESP_Score`, 0–100)

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

### 5.6 Dispute History Score (`DISP_Score`, 0–100)

```text
DISP_Score = max(0, 100 - Dispute_Penalty)

Dispute_Penalty = (Disputes_Filed_Against × Severity_Factor)
                  + (Disputes_Lost × 15)
                  + Pattern_Penalty
```

| Severity | Factor |
|---|---|
| Resolved amicably | 5 |
| Mediated resolution | 10 |
| Lost arbitration | 20 |

| Pattern | Penalty |
|---|---|
| 3+ disputes in 6 months | +10 |
| 5+ disputes in 6 months | +25 |

### 5.7 Platform Longevity Score (`AGE_Score`, 0–100)

```text
AGE_Score = min(100, account_age_days / 10)
```

Maximum at ~2.7 years (1,000 days). Inactivity decay: `-1` point per 30 days of inactivity after 90 days of no deal/message activity.

### 5.8 Community Contribution Score (`COMM_Score`, 0–100)

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

## 6. Role-Specific Scores

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

---

## 7. Trust Score Lifecycle

### 7.1 Events that trigger recalculation

| Event | Affected components | Action |
|---|---|---|
| Deal completed | TX, REV, AGE | Recalculate; request reviews from all parties |
| Deal cancelled | TX, DISP | Recalculate; apply cancellation penalty |
| Deal disputed | TX, DISP | Recalculate; flag for moderation if pattern emerges |
| Review submitted | REV | Recalculate weighted average |
| Review challenged / resolved | REV, DISP | Recalculate if review is removed or modified |
| Profile updated | PROF | Recompute profile completeness |
| Verification approved | VER, PROF | Recompute verification level |
| Message sent/read | RESP | Update response rate |
| Dispute resolved | DISP | Apply penalties/rewards |
| Nightly job | AGE, RESP, COMM, decay | Recalculate time-based components |

### 7.2 Recalculation flow

```text
Event occurs
    │
    ▼
┌─────────────────────────────┐
│ Load Party + trust_scores row│
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
└─────────────────────────────┘
    │
    ▼
┌─────────────────────────────┐
│ Tier changed?                │
│ → notify, update fee tier,   │
│   log audit                  │
└─────────────────────────────┘
```

### 7.3 Decay rules

- **Inactivity decay:** after 180 days with no deal/message activity, the overall score decays by `2` points per month until activity resumes.
- **Review recency:** older reviews automatically lose weight through the `Recency_j` factor.
- **Penalty expiry:** cancellation/no-show penalties remain for 12 months, then are halved; they fully expire after 24 months.

---

## 8. Transactions Feature

### 8.1 Philosophy

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

### 8.2 Wallet model

Each Party has one `platform_wallets` row:

| Field | Meaning |
|---|---|
| `balance` | Points available for withdrawal or new deals |
| `escrow_balance` | Points held in escrow for active deals |
| `pending_balance` | Points awaiting transaction approval |
| `total_deposited` | Cumulative deposits |
| `total_withdrawn` | Cumulative withdrawals |

### 8.3 Transaction types

| Type | Description |
|---|---|
| `DEPOSIT` | Funds added to a Party's wallet |
| `WITHDRAWAL` | Funds removed from a Party's wallet |
| `ESCROW_HOLD` | Funds moved from wallet balance to escrow |
| `ESCROW_RELEASE` | Funds released from escrow to a recipient's wallet |
| `FEE` | Platform fee deducted |
| `ADJUSTMENT` | Refund, reversal, or manual correction |
| `IN_KIND` | Non-monetary value recorded for audit |

### 8.4 Transaction statuses

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

### 8.5 Multi-party approval

Every transaction with `requires_approval = true` must be approved by every Party referenced in `from_party_id` and `to_party_id`.

Rules:

1. `approvals_required` is set to the number of distinct involved parties when the transaction is created.
2. Any active member of an involved Party may submit an approval on behalf of that Party.
3. A Party can only approve once per transaction (`UNIQUE (transaction_id, party_id)`).
4. Decision is either `APPROVED` or `REJECTED`.
5. If any involved Party rejects, the transaction moves to `REJECTED` and the ledger is not mutated.
6. When all involved Parties approve, the transaction moves to `VERIFIED` and the ledger mutation is applied atomically.

### 8.6 Ledger mutation rules

| Transaction type | From wallet | To wallet | Escrow impact |
|---|---|---|---|
| `DEPOSIT` | — | `to_party_id` | `balance += amount` |
| `WITHDRAWAL` | `from_party_id` | — | `balance -= amount` |
| `ESCROW_HOLD` | `from_party_id` | — | `balance -= amount`, `escrow_balance += amount` |
| `ESCROW_RELEASE` | — | `to_party_id` | `escrow_balance -= amount`, `to.balance += amount` |
| `FEE` | `from_party_id` | platform | `balance -= amount` or `escrow -= amount` |
| `ADJUSTMENT` | context-specific | context-specific | recorded with `description` |

All mutations are performed inside a single database transaction together with the final approval insert.

### 8.7 External settlement tracking

Because the platform does not hold fiat, each transaction records how the physical settlement was performed:

```text
payment_method: BANK_TRANSFER | CASH | CARD | CRYPTO | IN_KIND | OTHER
external_reference: "wire-ref-12345" | "receipt-abc" | null
```

These fields are for audit, dispute resolution, and future reconciliation with external payment providers.

---

## 9. Database Schema (Existing)

The following tables are already created by the existing migrations and are used as-is:

### 9.1 `trust_scores`

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

### 9.2 `platform_wallets`

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

### 9.3 `transactions`

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

### 9.4 `transaction_approvals`

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

---

## 10. Domain & Application Additions

To implement this document in the existing hexagonal codebase, the following modules are recommended (no existing code is changed):

### 10.1 `crates/domain`

```text
domain/src/entities/
  ├── trust_score.rs          # TrustScore, TrustTier, ScoreComponent
  ├── wallet.rs               # PlatformWallet
  └── transaction.rs          # Transaction, TransactionType, TransactionStatus, TransactionApproval

domain/src/repositories/
  ├── trust_score_repository.rs
  ├── wallet_repository.rs
  └── transaction_repository.rs

domain/src/services/
  └── trust_calculator.rs     # Pure function: inputs -> TrustScore
```

### 10.2 `crates/application`

```text
application/src/trust/
  ├── get_trust_score.rs
  ├── recalculate_trust_score.rs
  └── dto.rs

application/src/payments/
  ├── deposit_points.rs
  ├── withdraw_points.rs
  ├── record_transaction.rs
  ├── approve_transaction.rs
  ├── reject_transaction.rs
  ├── list_pending_approvals.rs
  └── dto.rs
```

### 10.3 `crates/infrastructure`

```text
infrastructure/src/repositories/
  ├── postgres_trust_score_repository.rs
  ├── postgres_wallet_repository.rs
  └── postgres_transaction_repository.rs
```

### 10.4 `crates/api`

```text
api/src/routes/
  ├── trust.rs
  └── payments.rs

api/src/handlers/
  ├── trust/
  └── payments/
```

---

## 11. API Contracts

### 11.1 Trust Score

#### Get trust score

```http
GET /api/v1/trust/party/{partyId}
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
  "scoreHistory": [
    { "date": "2026-04-01", "score": 68 },
    { "date": "2026-05-01", "score": 70 },
    { "date": "2026-06-01", "score": 72 }
  ],
  "lastCalculatedAt": "2026-06-14T00:00:00Z",
  "nextCalculationAt": "2026-06-15T00:00:00Z",
  "calculationFormula": { /* weights & version */ }
}
```

#### Force recalculation (admin or owner)

```http
POST /api/v1/trust/party/{partyId}/recalculate
Authorization: Bearer <jwt>
```

### 11.2 Wallets

#### Get my wallet

```http
GET /api/v1/payments/wallets/me
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Success `200`:

```json
{
  "walletId": "wallet-uuid-1",
  "partyId": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "balance": 18500.00,
  "escrowBalance": 0.00,
  "pendingBalance": 2000.00,
  "totalDeposited": 75000.00,
  "totalWithdrawn": 56500.00,
  "currency": "POINTS",
  "isActive": true,
  "createdAt": "2026-01-15T10:30:00Z",
  "updatedAt": "2026-06-14T10:05:00Z"
}
```

### 11.3 Transactions

#### List transactions for a wallet

```http
GET /api/v1/payments/wallets/me/transactions?status=PENDING&dealId=...
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

#### Record a transaction

```http
POST /api/v1/payments/transactions
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json

{
  "dealId": "550e8400-e29b-41d4-a716-446655440000",
  "milestoneId": "ms-uuid-1",
  "transactionType": "ESCROW_RELEASE",
  "toPartyId": "enhancer-party-uuid-1",
  "amount": 9000.00,
  "description": "Milestone payment: Soil Preparation & Planting Complete",
  "paymentMethod": "BANK_TRANSFER",
  "externalReference": "wire-ref-12345"
}
```

Success `201` with the created transaction in `PENDING` status.

#### Approve a transaction

```http
POST /api/v1/payments/transactions/{txnId}/approve
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json

{
  "comment": "Confirmed receipt of bank transfer"
}
```

#### Reject a transaction

```http
POST /api/v1/payments/transactions/{txnId}/reject
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json

{
  "comment": "Amount does not match agreement"
}
```

#### List pending approvals for my party

```http
GET /api/v1/payments/transactions/pending-approvals
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

---

## 12. Integration with the Deal Lifecycle

| Deal state | Trust action | Transaction action |
|---|---|---|
| `TERMS_LOCKED → COMMITTED` | — | Consumer `DEPOSIT` / `ESCROW_HOLD` for total deal value |
| `COMMITTED → EXECUTING` | — | Escrow confirmed, milestones enabled |
| Milestone verified | — | `ESCROW_RELEASE` transaction created in `PENDING`; supplier/enhancer approve |
| `EXECUTING → COMPLETED` | Increase completed deal counts; request reviews | Final releases completed |
| Deal cancelled | Increase cancelled count; apply penalty | `ADJUSTMENT` refund transaction |
| Deal disputed | Increase disputed count | Freeze related transactions until resolved |
| Review submitted | Recalculate review component | — |

---

## 13. Worked Example

**Party:** Green Acres Farm Ltd  
**Historical deals:** 12 completed, 1 cancelled, 0 disputed  
**Total completed value:** 90,000 points  
**Reviews:** 18 reviews, weighted average rating 4.5  
**Profile completeness:** 95%  
**Verification level:** 4  
**Response rate:** 98% within 4 hours  
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

Result: **Gold tier**, overall score **78**.

---

## 14. Error Mapping

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

## 15. Open Points & Future Extensions

- **Market-benchmark provider:** integrate external pricing data to weight value-bonus more accurately.
- **Machine-learning risk score:** add a predicted default-risk component with human-review safeguards.
- **Graph trust network:** score parties based on the trustworthiness of their repeated partners.
- **Real payment provider bridge:** keep the `payment_method`/`external_reference` fields but optionally settle via Stripe/Flutterwave instead of purely mirroring external settlement.
- **Smart-contract settlement:** for blockchain-ready domains, emit settlement instructions from approved transactions.
- **Party-group governance:** group-level trust score derived from member scores and group deal history.

---

## 16. Glossary

| Term | Meaning |
|---|---|
| **Trust Score** | Composite reputation metric per party, 0–100, derived from deals, reviews, verification, and behaviour. |
| **Role Score** | Trust score scoped to a single deal role: Supplier, Consumer, or Enhancer. |
| **Points** | Platform-internal unit of value; transactions mirror external physical settlements. |
| **Platform Wallet** | A Party's ledger record: available balance, escrow balance, and pending balance. |
| **Transaction Approval** | Approval by every party involved in a transaction before it is marked verified/complete. |
| **Trust Tier** | Bronze / Silver / Gold / Platinum classification based on overall score. |
| **Decay** | Scheduled reduction of score for inactivity or ageing penalties. |
