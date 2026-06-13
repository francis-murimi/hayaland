# Party Guide — Hayaland 3-Party Deal Platform

> **Scope:** This document describes everything related to **Party** entities in the 3-Party Deal Platform. A Party is any participant that can enter into a deal. It covers party types, roles, profiles, groups, verification, trust scores, user-to-party relationships, authorization, lifecycle rules, and API contracts.
>
> **Based on:** the `3partydeal.pdf` Software Design Document and the planned `hayaland` implementation.

---

## 1. What Is a Party?

A **Party** is the central business identity on the platform. It represents any individual, organization, or collective that participates in a deal.

Key distinctions:

| Concept | Purpose | Example |
|---|---|---|
| **User** | Authentication/identity account. Can log in, has credentials, JWT tokens. | `alice@example.com` |
| **Party** | Business entity that enters deals, owns resources, expresses needs, provides services, receives payments, builds reputation. | *Green Acres Farm Ltd* |
| **PartyRole** | The function a Party plays within a specific deal. | `SUPPLIER`, `CONSUMER`, `ENHANCER` |

A single User account may be associated with multiple Parties (e.g., a person who owns a farm and also runs a logistics company). A Party may be associated with multiple User accounts (e.g., a company where several employees can act on its behalf).

When calling deal-related APIs, the caller selects which Party they are acting as via the `X-Party-ID` header.

---

## 2. Party Types

Every Party has a `partyType` that determines its structural form.

| Party Type | Description | Use Case |
|---|---|---|
| `INDIVIDUAL` | A single natural person acting as a business participant. | A freelance consultant, a small farmer, a vehicle owner. |
| `ORGANIZATION` | A registered business or legal entity. | A grocery chain, a construction firm, a manufacturing plant. |
| `PARTY_GROUP` | A composite entity made up of multiple individuals or organizations pooling resources or needs and acting as a single Party. | A farmers cooperative, a corporate alliance, an informal resource pool. |

### 2.1 Party Type Rules

- A Party's type is set at creation and cannot be changed.
- Only `PARTY_GROUP` has additional `PartyGroup` metadata (governance model, members, voting rules).
- All three types can participate in deals identically once they have the required role profile.
- Tax/VAT identifiers (`taxId`) are optional.

---

## 3. Party Roles in Deals

A Party can act in one or more platform roles, but **only one role per deal**. The three deal roles are:

### 3.1 Supplier

A Party that offers an underutilized resource or asset.

**Examples of supplier contributions:**
- Idle farmland
- Vacant buildings or warehouse space
- Unused vehicles or fleet capacity
- Idle machinery or equipment
- Raw materials
- Data or intellectual property

**Supplier profile fields:**

| Field | Type | Description |
|---|---|---|
| `resourceTypes` | `UUID[]` | Categories of resources typically supplied. |
| `typicalCapacity` | String | Usual volume/capacity description (e.g., *10–50 acres*). |
| `availabilitySchedule` | JSON | When resources are available. |
| `preferredCompensation` | Enum[] | `FIXED_FEE`, `REVENUE_SHARE`, `HYBRID`, `IN_KIND`. |
| `insuranceVerified` | Boolean | Whether liability insurance is verified. |

### 3.2 Consumer

A Party that desires a specific output, product, or service.

**Examples of consumer needs:**
- Crop produce for a grocery chain
- Rental space for a retailer
- Transportation capacity for a logistics company
- Processed goods for a manufacturer
- Insights/analytics for a technology company

**Consumer profile fields:**

| Field | Type | Description |
|---|---|---|
| `needCategories` | `UUID[]` | Categories of needs typically expressed. |
| `typicalVolume` | String | Usual order volume description. |
| `preferredQualityStandard` | String | Quality expectations (e.g., *USDA Organic*). |
| `budgetRangeMin` | Decimal | Minimum budget. |
| `budgetRangeMax` | Decimal | Maximum budget. |
| `preferredPaymentTerms` | Enum[] | `UPFRONT`, `MILESTONE`, `ON_DELIVERY`, `DEFERRED`. |

### 3.3 Enhancer

A Party that bridges the gap between Supplier and Consumer by providing enabling input, expertise, or services.

**Examples of enhancer contributions:**
- Agrodealer providing seeds, fertilizer, and expertise
- Renovation contractor preparing a vacant building
- Mechanic maintaining a vehicle fleet
- Quality certifier or processor
- ML engineer analyzing data

**Enhancer profile fields:**

| Field | Type | Description |
|---|---|---|
| `enhancementTypes` | `UUID[]` | Types of enhancements offered. |
| `skills` | String[] | Specific skills or certifications. |
| `certifications` | JSON | Verified credentials. |
| `hourlyRate` | Decimal | Standard hourly rate if applicable. |
| `fixedRate` | Decimal | Standard fixed fee if applicable. |
| `equipmentOwned` | String[] | Equipment/tools available. |
| `availability` | JSON | Schedule of availability. |
| `typicalEngagementDuration` | String | Typical time to complete enhancement. |

### 3.4 Role Assignment Rules

- A Party must have at least one active role to participate in deals.
- A Party can hold all three roles (`SUPPLIER`, `CONSUMER`, `ENHANCER`) simultaneously, but each role has its own profile.
- A role can be removed only if the Party has no active deals in that role.
- Role profiles can be updated independently.
- When a Party initiates or joins a deal, the platform validates that the Party has the requested role active.

---

## 4. Party Core Attributes

| Attribute | Type | Constraints | Description |
|---|---|---|---|
| `partyId` | UUID | PK, immutable | Unique platform identifier. |
| `partyType` | Enum | `INDIVIDUAL`, `ORGANIZATION`, `PARTY_GROUP` | Classification of the party structure. |
| `displayName` | String | 3–120 chars, required | Public-facing name or brand. |
| `email` | String | Unique, validated | Primary contact email. |
| `phone` | String | Optional, E.164 format | Primary contact phone. |
| `taxId` | String | Optional, encrypted at rest | Tax/VAT identifier for invoicing. |
| `verificationStatus` | Enum | `UNVERIFIED`, `PENDING`, `VERIFIED`, `REJECTED` | KYC/Business verification state. |
| `primaryDomain` | UUID | FK → Category | Default domain of operation. |
| `location` | GeoPoint | lat/long + resolved address | Primary operational location. |
| `serviceRadiusKm` | Decimal | Nullable, ≥ 0 | Geographic service reach. |
| `trustScore` | Decimal | 0.00 – 5.00, derived | Current composite trust score. |
| `totalDealsCompleted` | Integer | ≥ 0, derived | Count of successfully completed deals. |
| `totalDealsInitiated` | Integer | ≥ 0, derived | Count of deals this party initiated. |
| `isActive` | Boolean | Default true | Soft-delete / suspension flag. |
| `createdAt` | Timestamp | Auto-set | Registration timestamp. |
| `updatedAt` | Timestamp | Auto-update | Last modification timestamp. |

### 4.1 Display Name Rules

- Must be 3–120 characters.
- Must be unique enough to avoid public confusion; exact uniqueness is not enforced globally but duplicate display names may be disambiguated in search.
- Cannot contain slurs or impersonate platform brands (enforced by moderation).

### 4.2 Email Rules

- Valid email format, normalized to lowercase.
- Must be unique across all Parties on the platform.
- A Party's email may differ from its associated User account email.

### 4.3 Location & Service Radius

- `location` includes latitude, longitude, and a structured address.
- `serviceRadiusKm` defines how far the Party is willing to operate from its primary location.
- Used by the matching engine for geographic fit scoring.

### 4.4 PostGIS Proximity & Radius Coverage

Party locations are stored as PostGIS `GEOGRAPHY(POINT, 4326)` so the platform can perform accurate, index-backed geospatial queries. The `serviceRadiusKm` field turns each Party into a service circle on the earth's surface.

#### Coverage model

A Party *covers* a target coordinate when the geodesic distance from the Party's primary location to the target is less than or equal to `serviceRadiusKm`:

```sql
ST_DWithin(
  parties.location_geo,
  ST_SetSRID(ST_MakePoint(target_longitude, target_latitude), 4326)::geography,
  parties.service_radius_km * 1000  -- metres
)
```

If `serviceRadiusKm` is `NULL`, the Party is treated as having no geographic restriction and can match anywhere (or be excluded from radius-filtered searches, depending on product policy).

#### Radius-filtered search

Search accepts explicit numeric parameters rather than a single composite string:

| Parameter | Type | Description |
|---|---|---|
| `lat` | Decimal | Target latitude (`-90` to `90`). |
| `lng` | Decimal | Target longitude (`-180` to `180`). |
| `radiusKm` | Decimal | How far from `(lat, lng)` to look. |

Example:

```http
GET /api/v1/parties/search?role=SUPPLIER&lat=37.7749&lng=-122.4194&radiusKm=50
```

The query planner uses the `GIST` index on `location_geo` when the filter is expressed as:

```sql
WHERE ST_DWithin(
  location_geo,
  ST_SetSRID(ST_MakePoint($lng, $lat), 4326)::geography,
  $radiusKm * 1000
)
```

#### Proximity ranking

For discovery and match scoring, results are ordered by distance from the target:

```sql
ORDER BY ST_Distance(
  location_geo,
  ST_SetSRID(ST_MakePoint($lng, $lat), 4326)::geography
)
```

The API response includes the computed distance:

```json
{
  "partyId": "...",
  "displayName": "Sunset Valley Farm",
  "distanceKm": 12.4,
  "withinServiceRadius": true
}
```

#### Fallback for non-PostGIS deployments

If PostGIS is unavailable, the schema falls back to plain `latitude` and `longitude` columns. The application layer can use the haversine formula for approximate radius filtering. This fallback is less accurate and cannot use a spatial index, so it should only be used in development or legacy environments.

#### Required migration

```sql
CREATE EXTENSION IF NOT EXISTS postgis;

ALTER TABLE parties
  ADD COLUMN location_geo GEOGRAPHY(POINT, 4326);

UPDATE parties
SET location_geo = ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)::geography
WHERE latitude IS NOT NULL AND longitude IS NOT NULL;

CREATE INDEX idx_parties_geo ON parties USING GIST(location_geo);
```

---

## 5. User-to-Party Relationship

A User can be linked to one or more Parties. Within each Party, the User has a membership role.

### 5.1 Membership Roles

| Membership Role | Permissions |
|---|---|
| `OWNER` | Full control over the Party. Can manage roles, profiles, members, groups, and delete/suspend the Party. |
| `ADMIN` | Can manage most Party data, invite members, update profiles, act on behalf of the Party in deals. Cannot delete the Party or transfer ownership. |
| `MEMBER` | Can view Party data and participate in deals if authorized. Cannot change core Party settings. |
| `OBSERVER` | Read-only access to Party data and deals. Cannot act on behalf of the Party. |

### 5.2 Rules

- A User must be a member of a Party to act as it in deal APIs.
- The `OWNER` is automatically created when a Party is created.
- Membership can be active or inactive.
- A User cannot be removed from a Party if they are the sole owner and the Party has active deals.

---

## 6. Party Groups

A `PARTY_GROUP` allows multiple Parties (individuals or organizations) to act as a single Party in a deal.

### 6.1 PartyGroup Attributes

| Attribute | Type | Constraints | Description |
|---|---|---|---|
| `groupId` | UUID | PK | Unique group identifier. |
| `partyId` | UUID | FK → Party, unique | Links to the parent Party record. |
| `groupName` | String | 3–200 chars | Name of the collective. |
| `groupType` | Enum | `COOPERATIVE`, `PARTNERSHIP`, `INFORMAL_POOL`, `CORPORATE_ALLIANCE` | Structural form. |
| `formationDate` | Date | Optional | When the group was formed. |
| `governanceModel` | String | Optional | Decision-making description. |
| `memberCount` | Integer | ≥ 1, derived | Current number of members. |
| `isOpenToNewMembers` | Boolean | Default false | Whether new members can join. |
| `minimumMemberApproval` | Integer | ≥ 1, ≤ memberCount | Votes needed for deal approval. |
| `sharedBankAccount` | Boolean | Default false | Whether finances are pooled. |

### 6.2 Group Membership

| Attribute | Type | Constraints | Description |
|---|---|---|---|
| `membershipId` | UUID | PK | Unique membership identifier. |
| `groupId` | UUID | FK → PartyGroup | The group being joined. |
| `memberPartyId` | UUID | FK → Party | The individual member. |
| `roleInGroup` | Enum | `LEADER`, `ADMIN`, `MEMBER`, `OBSERVER` | Authority level within the group. |
| `joinedAt` | Timestamp | Auto-set | Membership start date. |
| `contributionPercentage` | Decimal | 0 – 100 | Share of contribution/reward. |
| `isActive` | Boolean | Default true | Whether membership is current. |

### 6.3 Governance Models

The platform supports configurable governance for PartyGroups:

| Model | Decision Rule | Typical Use |
|---|---|---|
| `ANY_ONE` | Any single member can approve actions. | Small informal pools with high trust. |
| `MAJORITY` | More than 50% of members must approve. | Cooperatives and partnerships. |
| `ALL` | Unanimous approval required. | High-stakes corporate alliances. |
| `THRESHOLD` | A fixed number of approvals required. | Configurable for specific bylaws. |
| `WEIGHTED` | Approval weighted by contribution percentage. | Investor-style structures. |

### 6.4 Group Deal Approval Workflow

1. A deal action is initiated on behalf of the PartyGroup.
2. The platform creates a pending group action (e.g., *Approve Deal*).
3. Members cast votes (`APPROVE` / `REJECT` / `ABSTAIN`).
4. Once the governance threshold is met, the action is executed or rejected.
5. All votes are recorded in the audit trail.

---

## 7. Party Lifecycle

### 7.1 States

A Party's lifecycle is simpler than a Deal's but still governed by rules:

```
[CREATED] → [UNVERIFIED] → [VERIFIED]
   ↓              ↓
[SUSPENDED]  [REJECTED]
   ↓
[DEACTIVATED]
```

| State | Description |
|---|---|
| `CREATED` | Party record created; profile incomplete. |
| `UNVERIFIED` | Party active but not KYC/business verified. Can participate in some deals with restrictions. |
| `VERIFIED` | KYC/business verification passed. Full platform access. |
| `REJECTED` | Verification failed. Cannot initiate deals; may respond to invitations depending on policy. |
| `SUSPENDED` | Temporarily restricted by platform or owner. Cannot participate in new deals; existing active deals continue under review. |
| `DEACTIVATED` | Soft-deleted. Cannot participate in deals; hidden from search. Only allowed if no active deals. |

### 7.2 Lifecycle Rules

- A Party can be deactivated only if it has no active deals (deals in `EXECUTING`, `NEGOTIATING`, `TERMS_LOCKED`, `COMMITTED`, etc.).
- Deactivating a Party does not delete historical deal records or reviews.
- A suspended Party cannot initiate deals or be matched, but existing committed deals may continue unless the platform intervenes.

---

## 8. Verification & KYC

Verification increases trust, unlocks higher deal values, and may reduce platform fees.

### 8.1 Verification Levels

| Level | Documents Required | Capabilities |
|---|---|---|
| `BASIC` | Email/phone confirmation | Browse, respond to invitations, low-value deals. |
| `STANDARD` | Identity + address proof | Initiate deals, medium-value transactions. |
| `PREMIUM` | Identity + address + business registration + bank account | High-value deals, escrow privileges, fee discounts. |

### 8.2 Verification Process

1. Party owner initiates verification, selecting level and provider.
2. Documents are uploaded and encrypted at rest.
3. Platform or third-party provider reviews documents.
4. Status moves to `VERIFIED` or `REJECTED`.
5. `verifiedAt` and `expiresAt` are recorded.

### 8.3 Document Types

- `IDENTITY` (passport, national ID, driver's license)
- `ADDRESS_PROOF` (utility bill, bank statement)
- `BUSINESS_REGISTRATION` (certificate of incorporation, tax registration)
- `INSURANCE` (liability or professional insurance)
- `BANK_ACCOUNT` (for payouts)

---

## 9. Trust Score & Reputation

Each Party has a computed `TrustScore` that affects matching priority, deal eligibility, and platform fees.

### 9.1 Trust Score Components

| Dimension | Weight | Description |
|---|---|---|
| Transaction history | 30% | Completed, cancelled, and disputed deals. |
| Reviews | 25% | Star ratings and feedback from other parties. |
| Profile completeness | 15% | How complete the Party profile is. |
| Verification level | 15% | Higher verification → higher score. |
| Response rate | 10% | Speed of responding to invitations and messages. |
| Longevity | 5% | Time since registration. |

### 9.2 Role-Specific Scores

In addition to an overall score, a Party has separate scores for each role it has played:

- `asSupplierScore`
- `asConsumerScore`
- `asEnhancerScore`

These are computed from deals where the Party acted in that role.

### 9.3 Trust Score Impact

- **Matching:** higher trust scores appear earlier in match results.
- **Validation:** deals involving only high-trust parties may pass validation more easily.
- **Fees:** trust tier affects the platform fee multiplier.
- **Risk checks:** parties with too many disputes or cancellations may be blocked from initiating deals.

---

## 10. Party-to-Deal Participation Rules

This section formalizes how Parties relate to Deals.

### 10.1 Multiplicity

- **Party → DealParticipation:** 1:N. A Party can participate in many Deals.
- **Deal → DealParticipation:** 1:3. Every Deal has exactly three participations, one per role.
- **Party → Deal (as initiator):** 1:N. A Party can initiate many Deals.

### 10.2 Role Flexibility

- A Party may play **different roles in different Deals**.
  - Example: Party A is a `SUPPLIER` in Deal 1 (providing farmland) and a `CONSUMER` in Deal 2 (buying transportation).
- A Party may **not** play more than one role in the same Deal.
  - A Deal must have three distinct Parties, one in each role.

### 10.3 Participation Status

Each DealParticipation has a status:

| Status | Meaning |
|---|---|
| `INVITED` | Party has been invited to the deal but has not responded. |
| `PENDING` | Party is reviewing the deal; response not finalized. |
| `ACCEPTED` | Party has accepted the deal invitation or acknowledged the proposal. |
| `DECLINED` | Party has declined to participate. |
| `WITHDRAWN` | Party withdrew after previously accepting. |

### 10.4 Initiator Rules

- Any role can initiate a deal.
- The initiator's Party is automatically set as `ACCEPTED` in the DealParticipation with `isInitiator = true`.
- The initiator chooses the other two Parties or leaves slots open for matching.

---

## 11. Authorization: Acting as a Party

### 11.1 X-Party-ID Header

Because a User may belong to multiple Parties, deal-related endpoints require the caller to specify which Party they are acting as:

```http
GET /api/v1/deals
Authorization: Bearer <jwt>
X-Party-ID: 7c9e6679-7425-40de-944b-e07fc1f90ae7
```

### 11.2 Header Resolution Rules

- If the User belongs to exactly one active Party and no `X-Party-ID` is provided, the system may default to that Party (convenience behavior).
- If the User belongs to multiple Parties, `X-Party-ID` is required.
- The User must be an active member of the specified Party.
- The Party must be active (`isActive = true`).

### 11.3 Scope Checks

In addition to selecting a Party, the User must have the required scope:

#### Regular party scopes

These apply to the Party the caller is acting as or owns:

| Scope | Capability |
|---|---|
| `parties:read` | Read party data you belong to or public party data. |
| `parties:write` | Create/update parties you own or are admin of. |

#### Admin party scopes

These apply across all parties and are typically held by platform administrators:

| Scope | Capability |
|---|---|
| `admin:parties` | List, read, update, suspend, reactivate, verify, or force-delete any party. |
| `admin:parties:read` | Read-only access to all parties (for support/moderation). |
| `admin:parties:write` | Modify any party's core data, roles, or verification status. |
| `admin:parties:delete` | Force-delete parties even with active deals (extreme action, heavily audited). |

Admins are subject to the same `X-Party-ID` requirement only when they are personally participating in a deal. For administrative actions, they act as the platform admin and use their admin scopes.

| Scope | Capability |
|---|---|
| `parties:read` | Read party data. |
| `parties:write` | Create/update party. |
| `deals:read` | Read deal data. |
| `deals:write` | Create/update deal. |
| `deals:transition` | Execute state transitions. |
| `terms:negotiate` | Propose/counter terms. |
| `payments:read` / `payments:write` | View/initiate payments. |
| `admin:parties` | Read/write/administrate any party (platform admin). |
| `admin:users` | Manage user accounts. |
| `admin:deals` | Oversee and moderate deals. |
| `admin:*` | All administrative functions. |

---

## 12. Party Search & Discovery

Parties can be searched and filtered to find potential deal partners.

### 12.1 Search Parameters

| Parameter | Type | Description |
|---|---|---|
| `q` | String | Free-text search on display name, description, location. |
| `role` | Enum[] | Filter by role: `SUPPLIER`, `CONSUMER`, `ENHANCER`. |
| `domainCategoryId` | UUID | Filter by primary domain category. |
| `verificationStatus` | String | `VERIFIED`, `UNVERIFIED`, etc. |
| `minTrustScore` | Decimal | Minimum trust score (0–5). |
| `lat` | Decimal | Target latitude for proximity search. |
| `lng` | Decimal | Target longitude for proximity search. |
| `radiusKm` | Decimal | Search radius around `(lat, lng)`. |
| `available` | Boolean | Only parties available for new deals. |

### 12.2 Search Response

Search results include public fields only:

```json
{
  "partyId": "search-result-uuid-1",
  "displayName": "Sunset Valley Farm",
  "partyType": "ORGANIZATION",
  "roles": ["SUPPLIER"],
  "trustScore": 4.80,
  "verificationStatus": "VERIFIED",
  "location": { "city": "San Jose", "region": "CA" },
  "primaryDomainName": "Agriculture",
  "matchScore": 0.92,
  "distanceKm": 12.4,
  "withinServiceRadius": true,
  "resourceSummary": "25 acres farmland, organic certified"
}
```

---

## 13. Party API Contracts

### 13.1 Party CRUD

#### Create Party

```http
POST /api/v1/parties
Authorization: Bearer <jwt>
```

Request body:

```json
{
  "partyType": "ORGANIZATION",
  "displayName": "Green Acres Farm Ltd",
  "email": "contact@greenacres.example.com",
  "phone": "+1-555-0123",
  "taxId": "12-3456789",
  "primaryDomain": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "location": {
    "latitude": 37.7749,
    "longitude": -122.4194,
    "addressLine1": "123 Farm Road",
    "city": "San Francisco",
    "region": "CA",
    "country": "US",
    "postalCode": "94102"
  },
  "serviceRadiusKm": 100,
  "roles": ["SUPPLIER", "CONSUMER"]
}
```

Validation rules:
- `partyType` required.
- `displayName` 3–120 characters.
- `email` valid and unique across platform.
- `roles` must contain at least one valid role.
- `location.latitude` and `location.longitude` within valid coordinate ranges.

#### Get Party

```http
GET /api/v1/parties/{partyId}
Authorization: Bearer <jwt>
```

Field visibility:
- Public fields visible to all authenticated users.
- Private fields (taxId, full membership list, documents) visible to party members and admins.

#### Update Party

```http
PUT /api/v1/parties/{partyId}
PATCH /api/v1/parties/{partyId}
Authorization: Bearer <jwt>
```

Authorization: Party owner, admin, or member with `parties:write` permission.

#### Delete Party (Soft Delete)

```http
DELETE /api/v1/parties/{partyId}?force=false
Authorization: Bearer <jwt>
```

Fails with `409 CONFLICT` if the Party has active deals.

### 13.2 Role Management

#### List Roles

```http
GET /api/v1/parties/{partyId}/roles
```

#### Add Role

```http
POST /api/v1/parties/{partyId}/roles
```

```json
{
  "roleType": "ENHANCER",
  "enhancerProfile": {
    "enhancementTypes": ["agricultural-consulting", "soil-analysis"],
    "skills": ["Precision Agriculture", "Organic Certification"],
    "hourlyRate": 150.00,
    "equipmentOwned": ["Soil Testing Kit"],
    "typicalEngagementDuration": "2-6 weeks"
  }
}
```

Returns `409 CONFLICT` if the role is already assigned.

#### Update Role Profile

```http
PUT /api/v1/parties/{partyId}/roles/{roleType}
```

#### Remove Role

```http
DELETE /api/v1/parties/{partyId}/roles/{roleType}?force=false
```

Fails if the Party has active deals in that role.

### 13.3 List My Parties

```http
GET /api/v1/parties/me
Authorization: Bearer <jwt>
```

Returns all Parties associated with the authenticated User.

```json
{
  "data": [
    {
      "partyId": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
      "displayName": "Green Acres Farm Ltd",
      "partyType": "ORGANIZATION",
      "roles": ["SUPPLIER", "CONSUMER"],
      "verificationStatus": "VERIFIED",
      "trustScore": 4.52,
      "memberRole": "OWNER",
      "isActive": true
    }
  ]
}
```

### 13.4 Search Parties

```http
GET /api/v1/parties/search?role=SUPPLIER&domainCategoryId=...&minTrustScore=4.0&lat=37.7749&lng=-122.4194&radiusKm=50
```

Query parameters:

| Parameter | Type | Description |
|---|---|---|
| `q` | String | Free-text search on display name / email. |
| `role` | Enum[] | Filter by active party role. |
| `partyType` | String[] | `INDIVIDUAL`, `ORGANIZATION`, `PARTY_GROUP`. |
| `verificationStatus` | String[] | `UNVERIFIED`, `PENDING`, `VERIFIED`, `REJECTED`. |
| `minTrustScore` | Decimal | Minimum trust score. |
| `maxTrustScore` | Decimal | Maximum trust score. |
| `lat` | Decimal | Target latitude for radius filter. |
| `lng` | Decimal | Target longitude for radius filter. |
| `radiusKm` | Decimal | Search radius in kilometres. Requires `lat` and `lng`. |
| `limit` / `offset` | Integer | Pagination. |

### 13.5 Nearby Parties

Dedicated endpoint for geographic discovery. Returns active parties whose primary location falls within the requested radius, ordered by distance from the target point.

```http
GET /api/v1/parties/nearby?lat=37.7749&lng=-122.4194&radiusKm=10&role=SUPPLIER
Authorization: Bearer <jwt>
```

`lat`, `lng`, and `radiusKm` are all required. The response uses the same shape as `/parties/search`.

### 13.6 Party Group Management

#### Create Party Group

```http
POST /api/v1/parties/{partyId}/group
```

Converts an existing Party into a `PARTY_GROUP`.

#### List Group Members

```http
GET /api/v1/parties/{partyId}/group/members
```

#### Add Group Member

```http
POST /api/v1/parties/{partyId}/group/members
```

```json
{
  "memberPartyId": "new-member-uuid",
  "roleInGroup": "MEMBER",
  "contributionPercentage": 15.0
}
```

#### Remove Group Member

```http
DELETE /api/v1/parties/{partyId}/group/members/{memberPartyId}
```

#### Vote on Group Action

```http
POST /api/v1/parties/{partyId}/group/members/{actionId}/vote
```

```json
{
  "vote": "APPROVE",
  "comment": "New member brings valuable expertise"
}
```

### 13.6 Verification APIs

#### Initiate Verification

```http
POST /api/v1/parties/{partyId}/verification/initiate
```

```json
{
  "verificationLevel": "PREMIUM",
  "provider": "onfido",
  "documents": [
    { "type": "IDENTITY", "documentUrl": "..." },
    { "type": "ADDRESS_PROOF", "documentUrl": "..." },
    { "type": "BUSINESS_REGISTRATION", "documentUrl": "..." }
  ]
}
```

#### Get Verification Status

```http
GET /api/v1/parties/{partyId}/verification
```

---

## 14. Party Database Schema

### 14.1 Core Tables

```sql
-- Enable PostGIS for GEOGRAPHY columns and GIST spatial indexes.
CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE parties (
    id UUID PRIMARY KEY,
    party_type TEXT NOT NULL CHECK (party_type IN ('INDIVIDUAL','ORGANIZATION','PARTY_GROUP')),
    display_name CITEXT NOT NULL,
    email CITEXT NOT NULL UNIQUE,
    phone TEXT,
    tax_id TEXT,
    verification_status TEXT NOT NULL DEFAULT 'UNVERIFIED',
    primary_domain_id UUID REFERENCES categories(id),
    -- Plain columns are kept as a fallback when PostGIS is not installed.
    latitude DOUBLE PRECISION,
    longitude DOUBLE PRECISION,
    -- PostGIS column is the source of truth for geospatial queries.
    location_geo GEOGRAPHY(POINT, 4326),
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
    member_role TEXT NOT NULL DEFAULT 'MEMBER' CHECK (member_role IN ('OWNER','ADMIN','MEMBER','OBSERVER')),
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

### 14.2 Group Tables

```sql
CREATE TABLE party_groups (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL UNIQUE REFERENCES parties(id) ON DELETE CASCADE,
    group_name TEXT NOT NULL,
    group_type TEXT NOT NULL CHECK (group_type IN ('COOPERATIVE','PARTNERSHIP','INFORMAL_POOL','CORPORATE_ALLIANCE')),
    formation_date DATE,
    governance_model TEXT,
    is_open_to_new_members BOOLEAN NOT NULL DEFAULT false,
    minimum_member_approval INTEGER NOT NULL DEFAULT 1,
    shared_bank_account BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE party_group_memberships (
    id UUID PRIMARY KEY,
    group_id UUID NOT NULL REFERENCES party_groups(id) ON DELETE CASCADE,
    member_party_id UUID NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
    role_in_group TEXT NOT NULL CHECK (role_in_group IN ('LEADER','ADMIN','MEMBER','OBSERVER')),
    contribution_percentage DECIMAL CHECK (contribution_percentage BETWEEN 0 AND 100),
    is_active BOOLEAN NOT NULL DEFAULT true,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (group_id, member_party_id)
);

CREATE TABLE party_group_actions (
    id UUID PRIMARY KEY,
    group_id UUID NOT NULL REFERENCES party_groups(id) ON DELETE CASCADE,
    action_type TEXT NOT NULL,
    target_id UUID,
    status TEXT NOT NULL DEFAULT 'PENDING',
    votes_required INTEGER NOT NULL,
    votes_received INTEGER NOT NULL DEFAULT 0,
    deadline TIMESTAMPTZ,
    executed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE party_group_votes (
    id UUID PRIMARY KEY,
    action_id UUID NOT NULL REFERENCES party_group_actions(id) ON DELETE CASCADE,
    member_party_id UUID NOT NULL REFERENCES parties(id),
    vote TEXT NOT NULL CHECK (vote IN ('APPROVE','REJECT','ABSTAIN')),
    comment TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (action_id, member_party_id)
);
```

### 14.3 Verification & Trust Tables

```sql
CREATE TABLE party_verifications (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
    verification_level TEXT NOT NULL,
    provider TEXT,
    provider_reference TEXT,
    status TEXT NOT NULL DEFAULT 'PENDING',
    documents JSONB NOT NULL DEFAULT '[]',
    verified_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
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
```

### 14.4 Indexes

```sql
CREATE INDEX idx_parties_email ON parties(email);
CREATE INDEX idx_parties_type ON parties(party_type);
-- Spatial GIST index enables fast radius and nearest-neighbour queries.
CREATE INDEX idx_parties_geo ON parties USING GIST(location_geo);
CREATE INDEX idx_parties_primary_domain ON parties(primary_domain_id);
CREATE INDEX idx_user_party_memberships_user ON user_party_memberships(user_id);
CREATE INDEX idx_user_party_memberships_party ON user_party_memberships(party_id);
CREATE INDEX idx_party_roles_party ON party_roles(party_id);
CREATE INDEX idx_party_group_members_group ON party_group_memberships(group_id);
CREATE INDEX idx_party_group_members_member ON party_group_memberships(member_party_id);
CREATE INDEX idx_trust_scores_party ON trust_scores(party_id);
```

---

## 15. Party Validation Rules

### 15.1 Creation/Update Validation

- `displayName` must be 3–120 characters.
- `email` must be valid and unique across all parties.
- `partyType` must be one of the allowed enum values.
- `location` coordinates must be valid.
- `serviceRadiusKm` must be ≥ 0 if provided.
- `roles` must contain at least one valid role.

### 15.2 Role Assignment Validation

- A Party cannot have duplicate roles.
- Role profile must match the role type (e.g., only `SupplierProfile` fields for `SUPPLIER`).
- Role removal blocked if active deals exist in that role.

### 15.3 Deactivation Validation

- A Party cannot be deactivated if it has any active deals.
- A PartyGroup cannot be deactivated if any member has active deals acting on behalf of the group.

### 15.4 Group Validation

- A PartyGroup must have at least one member.
- `minimumMemberApproval` must be between 1 and `memberCount`.
- Sum of `contributionPercentage` across active members should equal 100 (validated as a warning, not a hard block, to allow flexible governance).

---

## 16. Admin Party Management

Platform administrators with the appropriate scopes can manage Parties across the platform. Admin actions are privileged, audited, and often bypass normal ownership checks.

### 16.1 Required Scopes

| Scope | Purpose |
|---|---|
| `admin:parties` | Full admin access to party management. |
| `admin:parties:read` | Read-only access for support and moderation. |
| `admin:parties:write` | Update party data, roles, verification status, suspend/reactivate. |
| `admin:parties:delete` | Force-delete parties, including those with active deals. |

A role definition such as `admin` should include `admin:*` or `admin:parties` to grant these capabilities.

### 16.2 Admin Capabilities

| Capability | Description | Owner vs Admin |
|---|---|---|
| **List all parties** | Paginated list of every party on the platform. | Admin only. |
| **Get any party** | Full read access to any party record, including private fields. | Admin only. |
| **Update any party** | Modify display name, email, phone, location, primary domain, etc. | Admin only (owners update their own). |
| **Suspend party** | Set `isActive = false` and prevent new deal participation. | Admin only. |
| **Reactivate party** | Set `isActive = true` after suspension or verification. | Admin only. |
| **Verify party** | Override verification status to `VERIFIED` or `REJECTED`. | Admin only (owners initiate, admins approve/override). |
| **Add/remove roles** | Assign or remove Supplier/Consumer/Enhancer roles on any party. | Admin can force; owner can manage their own if no active deals. |
| **Force delete party** | Delete a party even if it has active deals. | Admin only; requires `admin:parties:delete` and is heavily audited. |
| **Manage group members** | Add/remove members from any PartyGroup or change governance. | Admin can override; group leaders normally manage. |
| **View audit history** | See all admin actions taken on a party. | Admin only. |

### 16.3 Admin API Endpoints

#### List all parties (admin)

```http
GET /api/v1/admin/parties
Authorization: Bearer <jwt-with-admin:parties>
```

Query parameters:

| Parameter | Type | Description |
|---|---|---|
| `status` | String[] | `ACTIVE`, `SUSPENDED`, `DEACTIVATED`. |
| `verificationStatus` | String[] | `UNVERIFIED`, `PENDING`, `VERIFIED`, `REJECTED`. |
| `partyType` | String[] | `INDIVIDUAL`, `ORGANIZATION`, `PARTY_GROUP`. |
| `q` | String | Free-text search on display name / email. |
| `limit` | Integer | Page size (default 20, max 100). |
| `cursor` | String | Pagination cursor. |

Response includes private/sensitive fields such as `taxId` and full verification documents (admin view).

#### Get any party (admin)

```http
GET /api/v1/admin/parties/{partyId}
Authorization: Bearer <jwt-with-admin:parties>
```

Returns full party record including:
- All role profiles
- Membership list with user IDs
- Verification history
- Trust score breakdown
- Active deal count
- Audit log of admin actions

#### Update any party (admin)

```http
PUT /api/v1/admin/parties/{partyId}
PATCH /api/v1/admin/parties/{partyId}
Authorization: Bearer <jwt-with-admin:parties>
```

Request body supports the same fields as the owner update endpoint, plus admin-only fields:

```json
{
  "displayName": "Updated Farm Name",
  "verificationStatus": "VERIFIED",
  "isActive": true,
  "adminNote": "Verified via manual review"
}
```

Admin updates bypass normal ownership checks but still enforce format validations (e.g., valid email, name length).

#### Suspend party

```http
POST /api/v1/admin/parties/{partyId}/actions/suspend
Authorization: Bearer <jwt-with-admin:parties>
```

```json
{
  "reason": "Suspicious activity under review",
  "suspendUntil": "2026-07-01T00:00:00Z"
}
```

Effects:
- `isActive` set to `false`.
- Party cannot initiate new deals.
- Party cannot be matched.
- Existing active deals may continue but are flagged for review.
- Notification sent to party owner.

#### Reactivate party

```http
POST /api/v1/admin/parties/{partyId}/actions/reactivate
Authorization: Bearer <jwt-with-admin:parties>
```

```json
{
  "reason": "Review complete, no violation found"
}
```

#### Verify or reject party (admin override)

```http
POST /api/v1/admin/parties/{partyId}/verification/status
Authorization: Bearer <jwt-with-admin:parties>
```

```json
{
  "status": "VERIFIED",
  "level": "PREMIUM",
  "reason": "Documents manually verified by support team",
  "expiresAt": "2028-06-13T00:00:00Z"
}
```

Allowed statuses: `VERIFIED`, `REJECTED`.

#### Add role to any party (admin)

```http
POST /api/v1/admin/parties/{partyId}/roles
Authorization: Bearer <jwt-with-admin:parties>
```

Same body as owner-initiated role addition, but admin can bypass the "active deals" restriction if `force=true` is passed (with audit log).

#### Remove role from any party (admin)

```http
DELETE /api/v1/admin/parties/{partyId}/roles/{roleType}?force=true
Authorization: Bearer <jwt-with-admin:parties>
```

#### Force delete party

```http
DELETE /api/v1/admin/parties/{partyId}?force=true
Authorization: Bearer <jwt-with-admin:parties>
```

Force deletion rules:
- Requires `admin:parties:delete` scope.
- Allowed even if the party has active deals, but all active deals must be cancelled or transferred first, or the admin must provide an explicit `terminateActiveDeals=true` flag.
- All related records (memberships, roles, group memberships, wallet) are either cascade-deleted or anonymized according to data retention policy.
- Action is irreversible and logged with admin ID and justification.

#### Get party admin audit log

```http
GET /api/v1/admin/parties/{partyId}/admin-history
Authorization: Bearer <jwt-with-admin:parties>
```

Returns a chronological list of admin actions on the party:

```json
{
  "data": [
    {
      "actionId": "evt-uuid-1",
      "actionType": "PARTY_SUSPENDED",
      "adminUserId": "admin-uuid",
      "adminEmail": "admin@hayaland.local",
      "reason": "Suspicious activity under review",
      "metadata": { "suspendUntil": "2026-07-01T00:00:00Z" },
      "createdAt": "2026-06-13T10:00:00Z"
    }
  ]
}
```

### 16.4 Admin-Only Response Fields

When an admin fetches a party, the response may include fields hidden from public/owner views:

| Field | Description |
|---|---|
| `internalNotes` | Platform moderation notes. |
| `suspensionReason` | Why the party was suspended. |
| `suspendedUntil` | When suspension expires, if applicable. |
| `verificationDocuments` | Full verification document metadata. |
| `adminActionCount` | Number of admin actions taken. |
| `riskFlags` | Automated or manual risk flags. |

### 16.5 Admin Action Rules

- **Principle of least privilege:** Admins should hold only the scopes they need (e.g., support staff get `admin:parties:read`, senior admins get `admin:parties:write`).
- **Audit everything:** Every admin mutation is recorded in `party_admin_history` with admin user ID, timestamp, reason, and before/after snapshot.
- **Sensitive actions require justification:** Suspend, force-delete, and verification override require a non-empty `reason`.
- **No self-dealing:** An admin cannot perform admin actions on a Party they personally own unless explicitly authorized by a second admin (optional dual-control policy).
- **Notifications:** Party owners are notified of admin actions that affect their ability to trade (suspension, role removal, deletion).

### 16.6 Data Retention for Deleted Parties

- Soft-deleted parties remain in the database for compliance and audit.
- Personal data may be anonymized after a retention period.
- Deals, reviews, and transactions involving the party are preserved for platform integrity.

---

## 18. Events Produced by Party Lifecycle

The Party context publishes domain events that other contexts may consume:

| Event | Trigger |
|---|---|
| `PartyCreated` | New party registered. |
| `PartyUpdated` | Party profile updated. |
| `PartyRoleAdded` | New role assigned to party. |
| `PartyRoleRemoved` | Role removed from party. |
| `PartyVerified` | Verification status changed to `VERIFIED`. |
| `PartyDeactivated` | Party soft-deleted. |
| `TrustScoreUpdated` | Trust score recalculated. |
| `PartyGroupMemberAdded` | Member joined a PartyGroup. |
| `PartyGroupMemberRemoved` | Member left or was removed. |
| `PartyGroupActionVoted` | Vote cast on a group action. |

---

## 19. Common Scenarios

### 17.1 Individual with Multiple Businesses

**Scenario:** Alice has a farm and a logistics company.

1. Alice registers one User account.
2. She creates Party A (`ORGANIZATION`) for *Green Acres Farm* with role `SUPPLIER`.
3. She creates Party B (`ORGANIZATION`) for *Alice's Haulage* with role `ENHANCER`/`CONSUMER`.
4. Alice uses `X-Party-ID: A` when acting as the farm, and `X-Party-ID: B` when acting as the haulage company.
5. Alice can be a Supplier in one deal and a Consumer/Enhancer in another.

### 17.2 Organization with Multiple Employees

**Scenario:** *FreshMart Grocery Chain* has a procurement team.

1. *FreshMart* is created as an `ORGANIZATION` Party by its owner.
2. The owner invites employees as `ADMIN` or `MEMBER` via `user_party_memberships`.
3. Any employee with appropriate membership and scope can act on behalf of *FreshMart* using `X-Party-ID: freshmart-id`.
4. Deal history records the acting User ID for audit purposes.

### 17.3 Cooperative as a Single Party

**Scenario:** A group of farmers forms a cooperative.

1. One farmer creates a `PARTY_GROUP` called *Valley Farmers Cooperative*.
2. Other farmers join as member Parties.
3. The cooperative acts as a single Supplier in deals.
4. Major actions (e.g., committing to a deal) require votes per the governance model.
5. Value distribution can be split among members based on contribution percentage.

---

## 20. Glossary

| Term | Definition |
|---|---|
| **Party** | An individual, organization, or group that participates in deals on the platform. |
| **PartyType** | Structural classification: `INDIVIDUAL`, `ORGANIZATION`, or `PARTY_GROUP`. |
| **PartyRole** | The function a Party plays in a specific deal: `SUPPLIER`, `CONSUMER`, or `ENHANCER`. |
| **Role Profile** | Role-specific attributes (e.g., `SupplierProfile`, `ConsumerProfile`, `EnhancerProfile`). |
| **User Party Membership** | The link between a User account and a Party, with a membership role (`OWNER`, `ADMIN`, `MEMBER`, `OBSERVER`). |
| **PartyGroup** | A composite Party made up of multiple member Parties with governance rules. |
| **Verification Status** | KYC/business verification state: `UNVERIFIED`, `PENDING`, `VERIFIED`, `REJECTED`. |
| **Trust Score** | Composite reputation metric (0–5) derived from reviews, history, verification, and behavior. |
| **X-Party-ID** | HTTP header used to select which Party a User is acting as. |
