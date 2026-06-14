# Agreement Generation & Signing — Hayaland 3-Party Deal Platform

> **Scope:** This document specifies how Hayaland converts a deal with locked terms into a formal, signed agreement, how the three parties (Supplier, Consumer, Enhancer) execute that agreement, and how the platform records signatures to move a deal from `TERMS_LOCKED` to `COMMITTED`.
>
> **Audience:** Backend engineers, product owners, frontend engineers, and API consumers building on the Hayaland deal lifecycle.
>
> **Based on:** `3partydeal.pdf` Software Design Document, `hayaland-deal-plan.md`, `deal-plan.md`, `negotiation-guide.md`, `party-guide.md`, and the existing `hayaland` Rust codebase.

---

## 1. Goals

1. **Formalise locked terms.** Once negotiation ends and terms are locked, the platform generates a human-readable agreement that captures the final terms, value distribution, milestones, and participant details.
2. **Collect evidence of consent.** Every party must sign the agreement before the deal becomes `COMMITTED`. A signature is an auditable attestation that the party accepts the agreement text.
3. **Gate the deal lifecycle.** The `TERMS_LOCKED → COMMITTED` transition is only allowed when a valid agreement exists and all three participating parties have signed it.
4. **Maintain auditability.** Agreement text, signature data, signing user, IP address, and timestamp are persisted immutably for dispute resolution and regulatory audit.
5. **Support renegotiation.** If parties return to `NEGOTIATING`, the existing agreement is superseded and a new version is generated when terms are locked again.
6. **Keep agreements private.** An agreement is visible only to active members of the three participating parties and to platform admins with `admin:deals` or `admin:*`. It is never publicly accessible.
7. **Enable platform oversight.** Admins can view any agreement and edit selected administrative fields (governing law, dispute resolution clause, platform response, status overrides) for moderation, support, and dispute resolution.

---

## 2. When Agreement Generation Happens

An agreement is generated automatically when a deal transitions into `TERMS_LOCKED`. It is not created manually by a user.

```text
DRAFT → SUGGESTED → PENDING_REVIEW → NEGOTIATING → TERMS_LOCKED
                                                            │
                                                            ▼
                                                  ┌───────────────────┐
                                                  │ Generate Agreement │
                                                  │   Version 1        │
                                                  └───────────────────┘
                                                            │
                                                            ▼
                                            ┌───────────────────────────────┐
                                            │ Parties sign in any order      │
                                            │ (Supplier, Consumer, Enhancer) │
                                            └───────────────────────────────┘
                                                            │
                                                            ▼
                                                   ┌────────────────┐
                                                   │  COMMITTED     │
                                                   └────────────────┘
```

Agreement generation is also triggered again if the deal returns to `TERMS_LOCKED` after renegotiation:

```text
TERMS_LOCKED → NEGOTIATING (renegotiation requested)
                    │
                    ▼
         Terms updated / value distribution updated
                    │
                    ▼
           TERMS_LOCKED again
                    │
                    ▼
         New agreement version generated
                    │
                    ▼
         All parties sign again
```

---

## 3. Agreement Visibility

Agreements inherit the same privacy rules as deals:

| Who can view? | Condition |
|---|---|
| Party members | Active member (`OWNER`, `ADMIN`, `MEMBER`, or `OBSERVER`) of any of the three participating parties. |
| Platform admins | Holds `admin:deals` or `admin:*` scope. |
| Public / other users | Never. |

This applies to:

- The agreement text
- Signature records
- Payment schedules and value distribution details embedded in the agreement
- Any attachments or rendered PDFs

A user acting on behalf of a party must provide the `X-Party-ID` header. If the user belongs to exactly one active party, the system may default to it; otherwise the header is required. Admins do not need `X-Party-ID` for view or admin-edit operations.

## 4. Domain Model

### 4.1 `Agreement` aggregate root

An agreement belongs to exactly one deal. The existing migration defines it as:

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Primary key. |
| `deal_id` | UUID | FK → deals. One agreement version per deal at any time. |
| `agreement_status` | Enum | `DRAFT`, `PENDING_SIGNATURES`, `SIGNED`, `EXECUTED`, `TERMINATED`. |
| `agreement_text` | Text | Full rendered agreement text (Markdown by convention). |
| `governing_law` | Text | Jurisdiction / governing law clause. |
| `dispute_resolution` | Text | Dispute resolution clause. |
| `effective_date` | Date | Date the agreement takes effect. |
| `termination_date` | Date | Optional expiration date. |
| `auto_renew` | Boolean | Whether the agreement auto-renews. |
| `version` | Integer | Starts at 1; incremented on every regeneration. |
| `digital_signature_url` | Text | Optional URL to a rendered signed PDF (future). |
| `created_at` | Timestamp | When this agreement version was generated. |
| `executed_at` | Timestamp | When the final signature was collected. |

### 4.2 `Signature` entity

A signature is a record that a specific party, acting through an authorised user, has accepted the agreement.

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Primary key. |
| `agreement_id` | UUID | FK → agreements. |
| `party_id` | UUID | FK → parties. The party that signed. |
| `signed_by_user_id` | UUID | FK → users. The user who performed the signature. |
| `signature_type` | Text | `DIGITAL_ATTESTATION`, `CLICKWRAP`, `ESIGN`, etc. |
| `signature_data` | Text | SHA-256 hash or attestation string. |
| `ip_address` | Text | IP address at time of signing. |
| `signed_at` | Timestamp | When the signature was recorded. |

The unique constraint `UNIQUE (agreement_id, party_id)` ensures a party signs an agreement version only once.

### 4.3 Agreement status lifecycle

```text
DRAFT → PENDING_SIGNATURES → SIGNED → EXECUTED
          │        │
          │        └─> Any party can renegotiate, which moves deal to NEGOTIATING
          │            and the agreement is superseded.
          │
          └─> If a deal is cancelled, the agreement moves to TERMINATED.
```

| Status | Meaning |
|---|---|
| `DRAFT` | Agreement text generated but not yet published to parties. Transient state. |
| `PENDING_SIGNATURES` | Agreement published; awaiting one or more signatures. |
| `SIGNED` | All three parties have signed; the deal can now move to `COMMITTED`. |
| `EXECUTED` | The deal has moved to `COMMITTED` and execution has started. |
| `TERMINATED` | The deal was cancelled or the agreement was superseded by a new version. |

### 4.4 Relationship to `Deal`

- `Deal` 1:1 `Agreement` (current version).
- `Agreement` 1:N `Signature`.
- `Agreement` N:1 `Deal` historically if multiple versions are retained (keep all versions, mark old ones `TERMINATED`).

---

## 5. Agreement Content

### 5.1 Inputs to the agreement renderer

The agreement text is generated from the following deal data:

| Source | Data |
|---|---|
| `Deal` | Title, reference, description, domain category, location, expected dates, platform fee. |
| `deal_participations` | Party IDs, roles, value shares, initiator flag. |
| `parties` | Display names, party types, locations. |
| `resources` | Supplier resource details. |
| `needs` | Consumer need details. |
| `enhancements` | Enhancer contribution details. |
| `terms` | All accepted mandatory and optional terms. |
| `value_distributions` | Total value, shares, payment schedule, model. |
| `milestones` | Milestone names, due dates, payment triggers, completion criteria. |
| Platform config | Governing law, dispute resolution text, platform fee policy. |

### 5.2 Agreement structure

The generated agreement text should contain the following sections:

```markdown
# 3-Party Deal Agreement

## Deal Reference
DL-2026-0001

## Parties
- **Supplier:** Green Acres Farm Ltd (ORGANIZATION)
- **Consumer:** FreshMart Grocery Chain (ORGANIZATION)
- **Enhancer:** AgriTech Solutions (ORGANIZATION)

## Purpose
[Description from deal.deal_description]

## Resource, Need, and Enhancement
[Rendered from resources, needs, enhancements tables]

## Terms and Conditions
[List of accepted terms with term_name, description, and version]

## Value Distribution
- Total deal value: 30,000 POINTS
- Supplier share: 60% (18,000 POINTS)
- Enhancer share: 30% (9,000 POINTS)
- Platform fee: 10% (3,000 POINTS)
- Consumer cost: 30,000 POINTS

## Payment Schedule
[List from value_distributions.payment_schedule]

## Milestones
[List from milestones table]

## Governing Law
[Configured governing law]

## Dispute Resolution
[Configured dispute resolution clause]

## Platform Fee Acknowledgment
The parties acknowledge the platform fee of X% set for this deal.

## Signature Block
Each party acknowledges that they have read, understood, and agreed to the terms above.
```

### 5.3 Versioning

- `agreement.version` is incremented every time a new agreement is generated for the same deal.
- A new agreement is generated whenever the deal re-enters `TERMS_LOCKED` from `NEGOTIATING`.
- Old agreement versions remain in the `agreements` table with status `TERMINATED`.
- Signatures are tied to a specific `agreement_id`, so signatures from a superseded version do not count toward the current version.

### 5.4 Non-repudiation

The agreement text and all accepted terms are frozen at the moment of generation. To prevent tampering:

- The agreement text is stored verbatim in `agreements.agreement_text`.
- A SHA-256 hash of the agreement text + deal ID + version can be stored in `signature_data` as the attestation.
- Each signature records the signing user, party, IP address, and timestamp.

---

## 6. Signing Process

### 6.1 Who can sign

Only active members of the participating party can sign on behalf of that party. Membership roles allowed to sign:

| Membership Role | Can sign? |
|---|---|
| `OWNER` | Yes |
| `ADMIN` | Yes |
| `MEMBER` | Only if explicitly granted `deals:sign` or similar scope (default: no) |
| `OBSERVER` | No |

A user must:
1. Be authenticated with a valid JWT.
2. Belong to the party they are signing for.
3. Act as that party via the `X-Party-ID` header.
4. Have the required scope (`deals:write` or `deals:sign`).

**Admins cannot sign on behalf of parties.** An admin signature is not a substitute for party consent. If an admin needs to intervene (e.g., a party is unresponsive and arbitration authorises it), that action is recorded as an `ADMIN_OVERRIDE` in `deal_history`, not as a party signature.

### 6.2 Signature types

For the MVP, the signature is a digital attestation, not a legally binding e-signature.

| Signature Type | Description |
|---|---|
| `DIGITAL_ATTESTATION` | SHA-256 hash of agreement text + party ID + timestamp + attestation string. |
| `CLICKWRAP` | User clicked "I agree"; record the click with timestamp and IP. |
| `ESIGN` | Future: integration with qualified e-signature provider. |

The MVP default is `DIGITAL_ATTESTATION`.

### 6.3 Attestation string format

```text
I, {user_name}, acting on behalf of {party_display_name} as {role},
have read and agree to Agreement {agreement_id} version {version}
for Deal {deal_reference} on {signed_at}.
```

The `signature_data` field stores:

```text
sha256(agreement_text + "\n" + party_id + "\n" + signed_by_user_id + "\n" + signed_at_rfc3339 + "\n" + attestation_string)
```

### 6.4 Signing flow

```text
Deal in TERMS_LOCKED
        │
        ▼
┌───────────────────────┐
│ Generate agreement v1  │
│ Status: PENDING_SIGNATURES │
└───────────────────────┘
        │
        ▼
┌───────────────────────┐
│ Notify all parties    │
│ that agreement is ready│
└───────────────────────┘
        │
        ▼
┌───────────────────────────────┐
│ Parties sign in any order     │
│ Each signature recorded with  │
│ user, party, timestamp, IP    │
└───────────────────────────────┘
        │
        ▼
┌───────────────────────────────┐
│ All 3 signatures collected?   │
└───────────────────────────────┘
        │
   Yes  │
        ▼
┌───────────────────────┐
│ Agreement status: SIGNED │
└───────────────────────┘
        │
        ▼
┌───────────────────────────────┐
│ Deal transition:              │
│ TERMS_LOCKED → COMMITTED      │
│ (after escrow funding check)  │
└───────────────────────────────┘
        │
        ▼
┌───────────────────────┐
│ Agreement status: EXECUTED │
└───────────────────────┘
```

### 6.5 Partial signature state

While signatures are pending, the API exposes which parties have signed and which have not:

```json
{
  "agreementId": "agr-uuid-1",
  "dealId": "deal-uuid-1",
  "status": "PENDING_SIGNATURES",
  "version": 1,
  "signatures": [
    { "partyId": "supplier-party-uuid", "signed": true, "signedAt": "2026-06-14T10:00:00Z" },
    { "partyId": "consumer-party-uuid", "signed": false },
    { "partyId": "enhancer-party-uuid", "signed": false }
  ],
  "signaturesReceived": 1,
  "signaturesRequired": 3
}
```

### 6.6 Re-signing after renegotiation

If the deal moves `TERMS_LOCKED → NEGOTIATING`:

1. The current agreement status changes to `TERMINATED`.
2. A new agreement version is generated when terms are locked again.
3. All parties must sign the new agreement version.
4. Previous signatures are not reused.

---

## 7. State Machine Integration

### 7.1 Precondition for `TERMS_LOCKED → COMMITTED`

Before the transition is allowed, the application layer must verify:

1. The deal status is `TERMS_LOCKED`.
2. A current agreement exists for the deal.
3. The agreement status is `SIGNED` (all three parties have signed).
4. Win-Win-Win validation passes with no unacknowledged warnings.
5. Value distribution is present and valid.
6. Escrow is funded (or funding is in progress, depending on product policy).
7. All mandatory milestones are defined.

### 7.2 Transition flow

```text
POST /api/v1/deals/{deal_id}/transitions
{
  "new_status": "COMMITTED",
  "reason": "All parties signed; escrow funded"
}
```

The `ExecuteDealTransition` use case should:

1. Load the deal aggregate.
2. Verify the transition is valid using `Deal::can_transition`.
3. Check that the acting party is a participant.
4. Load the current agreement and verify it is `SIGNED`.
5. Re-run Win-Win-Win validation.
6. If all preconditions pass, update the deal status.
7. Set `agreement.executed_at` and `agreement.agreement_status = 'EXECUTED'`.
8. Record a `deal_history` event with actor, before/after status, and agreement version.

### 7.3 Agreement status transitions

| From | To | Trigger |
|---|---|---|
| `DRAFT` | `PENDING_SIGNATURES` | Agreement generated and published. |
| `PENDING_SIGNATURES` | `SIGNED` | Third signature received. |
| `SIGNED` | `EXECUTED` | Deal transitions to `COMMITTED`. |
| `PENDING_SIGNATURES` | `TERMINATED` | Deal moves back to `NEGOTIATING` or is cancelled. |
| `SIGNED` | `TERMINATED` | Deal is cancelled before `COMMITTED`. |

---

## 8. Database Schema (Existing)

The following tables are already defined in `migrations/20260613014000_create_agreements_signatures_reviews_trust.sql`:

### 8.1 `agreements`

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

### 8.2 `signatures`

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
    UNIQUE (agreement_id, party_id)
);
```

### 8.3 Notes on the current schema

- `agreements.deal_id` has a `UNIQUE` constraint, meaning only one agreement row per deal at the database level.
- To support multiple versions, the application must update the existing row in place OR the schema should be relaxed to allow multiple agreement versions. **Recommended approach:** keep one current agreement per deal (the row in `agreements`), move superseded versions to a separate `agreement_versions` table, or archive the old text in `deal_history`.
- For the MVP, the simplest approach is to update the existing agreement row when renegotiating and record the previous version text in `deal_history.details`.

---

## 9. Domain & Application Additions

To implement this document in the existing hexagonal codebase, the following modules are recommended (no existing code is changed):

### 9.1 `crates/domain`

```text
domain/src/entities/
  ├── agreement.rs        # Agreement, AgreementStatus
  └── signature.rs        # Signature, SignatureType

domain/src/repositories/
  └── agreement_repository.rs
```

### 9.2 `crates/application`

```text
application/src/agreements/
  ├── generate_agreement.rs   # GenerateAgreement use case
  ├── sign_agreement.rs       # SignAgreement use case
  ├── get_agreement.rs        # GetAgreement use case
  └── dto.rs                  # AgreementResult, SignatureResult
```

### 9.3 `crates/infrastructure`

```text
infrastructure/src/repositories/
  └── postgres_agreement_repository.rs
```

### 9.4 `crates/api`

```text
api/src/routes/
  └── agreements.rs

api/src/handlers/
  └── agreements/
      ├── get_agreement.rs
      └── sign_agreement.rs
```

---

## 10. API Contracts

### 10.1 Get agreement

```http
GET /api/v1/deals/{deal_id}/agreement
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

**Authorisation:** party participant (active member of any participating party) or platform admin (`admin:deals` / `admin:*`). Admins do not need an `X-Party-ID`. Non-participants receive `404` (not `403`) to avoid leaking deal existence.

Success `200`:

```json
{
  "agreementId": "agr-uuid-1",
  "dealId": "deal-uuid-1",
  "dealReference": "DL-2026-0001",
  "agreementStatus": "PENDING_SIGNATURES",
  "version": 1,
  "agreementText": "# 3-Party Deal Agreement\n\n## Parties...",
  "governingLaw": "State of California, USA",
  "disputeResolution": "Binding arbitration under AAA rules",
  "effectiveDate": "2026-07-01",
  "terminationDate": null,
  "autoRenew": false,
  "signatures": [
    {
      "signatureId": "sig-uuid-1",
      "partyId": "supplier-party-uuid",
      "partyDisplayName": "Green Acres Farm Ltd",
      "signedByUserId": "user-uuid-1",
      "signatureType": "DIGITAL_ATTESTATION",
      "signedAt": "2026-06-14T10:00:00Z"
    }
  ],
  "pendingSignatures": [
    {
      "partyId": "consumer-party-uuid",
      "partyDisplayName": "FreshMart Grocery Chain"
    },
    {
      "partyId": "enhancer-party-uuid",
      "partyDisplayName": "AgriTech Solutions"
    }
  ],
  "signaturesReceived": 1,
  "signaturesRequired": 3,
  "createdAt": "2026-06-14T09:00:00Z",
  "executedAt": null
}
```

### 10.2 Sign agreement

```http
POST /api/v1/deals/{deal_id}/agreement/sign
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json

{
  "signatureType": "DIGITAL_ATTESTATION",
  "attestation": "I agree to the terms of this agreement."
}
```

Success `200`:

```json
{
  "signatureId": "sig-uuid-2",
  "agreementId": "agr-uuid-1",
  "partyId": "consumer-party-uuid",
  "signedByUserId": "user-uuid-2",
  "signatureType": "DIGITAL_ATTESTATION",
  "signatureData": "sha256:abc123...",
  "signedAt": "2026-06-14T10:30:00Z"
}
```

If this was the final signature, the response also includes:

```json
{
  "agreementStatus": "SIGNED",
  "allPartiesSigned": true
}
```

### 10.3 Admin update agreement metadata

```http
PATCH /api/v1/admin/deals/{deal_id}/agreement
Authorization: Bearer <jwt>
Content-Type: application/json

{
  "governingLaw": "State of New York, USA",
  "disputeResolution": "Mediation followed by binding arbitration under AAA rules",
  "platformResponse": "Clause updated per platform policy 2026-06-14",
  "status": "TERMINATED",
  "reason": "Arbitration ruling #ARB-2026-0042"
}
```

**Authorisation:** platform admin (`admin:deals` or `admin:*`).

**Editable fields:**

| Field | Editable by admin? | Notes |
|---|---|---|
| `governingLaw` | Yes | Administrative clause, does not change party obligations. |
| `disputeResolution` | Yes | Administrative clause, typically updated after a dispute ruling. |
| `platformResponse` | Yes | Public or internal note added by platform support. |
| `status` | Yes, limited | Only to `TERMINATED` (e.g., after dispute or fraud finding) or back to `PENDING_SIGNATURES` (rare, heavily audited). |
| `agreementText` | No | Never edited directly; supersede by generating a new version if terms change. |
| `version` | No | System-managed. |

Success `200` with the updated agreement.

**Audit requirement:** every admin update creates a `deal_history` event of type `AGREEMENT_ADMIN_EDIT` recording the admin user ID, changed fields, before/after values, and reason.

### 10.4 Admin get any agreement

```http
GET /api/v1/admin/deals/{deal_id}/agreement
Authorization: Bearer <jwt>
```

**Authorisation:** platform admin (`admin:deals` or `admin:*`).

Returns the same payload as `GET /api/v1/deals/{deal_id}/agreement` but bypasses party-membership checks.

### 10.5 Transition to committed

```http
POST /api/v1/deals/{deal_id}/transitions
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json

{
  "new_status": "COMMITTED",
  "reason": "All parties signed; escrow funded"
}
```

Success `200` with the updated deal.

---

## 11. Use Case Specifications

### 11.1 `GenerateAgreement`

**Input:** `deal_id`

**Steps:**

1. Load the deal aggregate (deal + participations + terms + value distribution + milestones + resources/needs/enhancements).
2. Verify the deal status is `TERMS_LOCKED`.
3. Verify all mandatory terms are accepted.
4. Verify a valid value distribution exists.
5. Render the agreement text from a template using the aggregate data.
6. Insert or update the `agreements` row for the deal.
7. Set `agreement_status = 'PENDING_SIGNATURES'`.
8. Increment `version` if updating an existing agreement.
9. Record a `deal_history` event of type `AGREEMENT_GENERATED`.
10. Notify all participating parties that the agreement is ready for signature.

**Output:** `AgreementResult`

### 11.2 `SignAgreement`

**Input:** `deal_id`, `actor_user_id`, `actor_party_id`, `signature_type`

**Steps:**

1. Load the deal aggregate.
2. Verify the deal status is `TERMS_LOCKED`.
3. Load the current agreement for the deal.
4. Verify the agreement status is `PENDING_SIGNATURES`.
5. Verify the acting party is one of the three participating parties.
6. Verify the user is an active member of the acting party with signing permission.
7. Verify the party has not already signed this agreement version.
8. Build the attestation string and compute `signature_data`.
9. Insert a `signatures` row.
10. Check whether all three parties have now signed.
11. If yes, update `agreement_status = 'SIGNED'`.
12. Record a `deal_history` event of type `AGREEMENT_SIGNED` with party and user IDs.
13. Notify the other parties of the new signature.

**Output:** `SignatureResult`

### 11.3 `GetAgreement`

**Input:** `deal_id`, `user_id`, optional `party_id`, `is_admin`

**Steps:**

1. Load the deal aggregate.
2. Enforce deal visibility: user must be an active member of a participating party OR have `admin:deals` / `admin:*`.
3. If the user is a member but does not provide a valid `X-Party-ID`, resolve their active membership to one of the participating parties.
4. Load the current agreement and its signatures.
5. Return the agreement text and signature status.

**Output:** `AgreementResult`

### 11.4 `AdminUpdateAgreement`

**Input:** `deal_id`, `admin_user_id`, optional updates (`governing_law`, `dispute_resolution`, `platform_response`, `status`), `reason`

**Steps:**

1. Verify the caller has `admin:deals` or `admin:*` scope.
2. Load the current agreement for the deal.
3. Validate requested status change is allowed (e.g., `TERMINATED` or `PENDING_SIGNATURES` only).
4. Capture the before-state snapshot.
5. Apply the permitted updates.
6. Insert a `deal_history` event of type `AGREEMENT_ADMIN_EDIT` with admin user ID, reason, before/after snapshot.
7. Return the updated agreement.

**Output:** `AgreementResult`

**Restrictions:**

- Admin cannot add, remove, or alter signatures.
- Admin cannot change `agreement_text`, `version`, `deal_id`, `created_at`, or `executed_at`.
- Admin cannot mark an agreement `SIGNED` or `EXECUTED`; those transitions happen only through party signatures and the deal state machine.
- Terminating an agreement should generally be accompanied by a deal transition to `CANCELLED` or `DISPUTED`.

---

## 12. Validation & Business Rules

### 12.1 Agreement generation rules

| Rule | Error |
|---|---|
| Deal must be in `TERMS_LOCKED` | `InvalidStateTransition` |
| All mandatory terms must be accepted | `Validation("mandatory terms not accepted")` |
| Value distribution must be present and valid | `InvalidValueDistribution` |
| Win-Win-Win validation must pass | `WinWinWinValidationFailed` |
| Exactly three distinct participations must exist | `Validation("incomplete participations")` |

### 12.2 Signing rules

| Rule | Error |
|---|---|
| Deal must be in `TERMS_LOCKED` | `InvalidStateTransition` |
| Agreement must be in `PENDING_SIGNATURES` | `Validation("agreement not ready for signing")` |
| Acting party must be a participant | `DealAccessDenied` |
| User must be an active member of the acting party | `DealAccessDenied` |
| User must have signing permission | `Forbidden` |
| Party must not have already signed this version | `Validation("already signed")` |

### 12.3 Commit transition rules

| Rule | Error |
|---|---|
| Deal must be in `TERMS_LOCKED` | `InvalidStateTransition` |
| Agreement must be in `SIGNED` | `Validation("agreement not signed")` |
| Win-Win-Win validation must still pass | `WinWinWinValidationFailed` |
| Any warnings must be acknowledged | `WinWinWinValidationFailed` |

---

## 13. Security & Audit Considerations

1. **Signature binding.** Each signature is bound to a specific agreement version. If terms change, a new agreement is generated and signatures are collected again.
2. **IP and timestamp capture.** Every signature records the request IP address and signing timestamp to support non-repudiation.
3. **Permission checks.** Signing is restricted to owners and admins of the party by default; members can be granted explicit signing scope.
4. **Agreement text immutability.** Once generated, the agreement text for a version is never modified. Superseded versions are terminated, not edited.
5. **Deal history logging.** Every agreement generation and signature is recorded in `deal_history` for audit.
6. **Visibility enforcement.** Agreement endpoints must check `is_party_member(deal_id, user_id)` or `is_admin` on every read. Non-participants receive `404` to avoid information leakage.
7. **Admin view access.** Platform admins with `admin:deals` or `admin:*` can view any agreement without being a party participant. This is required for support, moderation, and dispute resolution.
7. **Admin edit restrictions.** Admins may edit only administrative metadata (`governing_law`, `dispute_resolution`, `platform_response`) and may terminate or reopen an agreement for signature in exceptional circumstances. They may not alter agreement text, versions, or signatures. Every admin edit is logged in `deal_history`.
8. **No admin signing.** Admins cannot sign on behalf of a party. If a party is unresponsive, the appropriate path is dispute resolution or termination, not admin signature.

---

## 14. Error Mapping

| Situation | Domain / Application error | HTTP status |
|---|---|---|
| Deal not found | `DealNotFound` | 404 |
| Agreement not found | `AgreementNotFound` / `NotFound` | 404 |
| Admin attempts to edit non-administrative field | `Validation` | 422 |
| Admin attempts to sign on behalf of a party | `Forbidden` | 403 |
| Non-admin attempts admin update endpoint | `Forbidden` | 403 |
| Deal not in `TERMS_LOCKED` | `InvalidStateTransition` | 409 |
| Agreement not in `PENDING_SIGNATURES` | `Validation` | 422 |
| User is not a member of a participating party and is not an admin | `DealNotFound` | 404 |
| Acting party not a participant | `DealAccessDenied` / `Forbidden` | 403 |
| User not a member of acting party | `DealAccessDenied` / `Forbidden` | 403 |
| Party already signed | `Validation` | 422 |
| Mandatory terms not accepted | `Validation` | 422 |
| Value distribution invalid | `InvalidValueDistribution` | 422 |
| Win-Win-Win validation fails | `WinWinWinValidationFailed` | 422 |

---

## 15. Worked Example

### 15.1 Scenario

- **Deal:** DL-2026-0001 — Organic Tomato Supply
- **Parties:**
  - Supplier: Green Acres Farm Ltd
  - Consumer: FreshMart Grocery Chain
  - Enhancer: AgriTech Solutions
- **Status:** `TERMS_LOCKED`
- **Terms:** Delivery date, quality standard, payment terms all accepted.
- **Value distribution:** 30,000 POINTS total; Supplier 60%, Enhancer 30%, Platform 10%.

### 15.2 Flow

1. System generates Agreement v1.
2. Agreement status: `PENDING_SIGNATURES`.
3. Supplier signs at 10:00 UTC.
4. Consumer signs at 10:30 UTC.
5. Enhancer signs at 11:00 UTC.
6. Agreement status changes to `SIGNED`.
7. Any party calls `POST /deals/{id}/transitions` with `COMMITTED`.
8. System verifies escrow funding and validation.
9. Deal transitions to `COMMITTED`.
10. Agreement status changes to `EXECUTED`.
11. Milestones are enabled and execution begins.

### 15.3 Superseded example

1. Deal is `TERMS_LOCKED`, Agreement v1 `PENDING_SIGNATURES`.
2. Consumer requests renegotiation.
3. Deal moves to `NEGOTIATING`.
4. Agreement v1 status changes to `TERMINATED`.
5. New quality term is proposed and accepted.
6. Deal moves back to `TERMS_LOCKED`.
7. System generates Agreement v2.
8. All parties must sign Agreement v2.

---

## 16. Open Points & Future Extensions

- **Legally binding e-signatures:** integrate with DocuSign, HelloSign, or qualified EU eIDAS providers.
- **Agreement templates per domain:** agriculture, real estate, transportation, etc.
- **Multi-language agreements:** render agreement text in the party's preferred language.
- **PDF generation:** produce a downloadable signed PDF stored in `digital_signature_url`.
- **Party group governance:** if a party is a `PARTY_GROUP`, require internal votes before signing.
- **Counter-signatures:** platform counter-signs the agreement as the facilitator.
- **Smart contract anchoring:** store agreement hash on a blockchain for additional non-repudiation.

---

## 17. Glossary

| Term | Meaning |
|---|---|
| **Agreement** | Formal rendered document capturing locked terms, value distribution, milestones, and signatures. |
| **Signature** | Auditable attestation that a party accepts an agreement version. |
| **Digital Attestation** | MVP signature type: SHA-256 hash of agreement text plus party/user/timestamp. |
| **Agreement Version** | Incremented each time a new agreement is generated for the same deal after renegotiation. |
| **Executed Agreement** | An agreement that has been signed by all parties and is bound to a `COMMITTED` deal. |
| **Superseded Agreement** | A previous agreement version terminated because terms were renegotiated. |
