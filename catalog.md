# Improved Market Catalogue Design — Public Discovery + Authenticated Actions

> **Status:** Design plan awaiting approval  
> **Scope:** Evolve the market catalogue so it is browsable by unauthenticated users, while keeping all mutating and contact actions authenticated. Add a first-class “contact owner” action that routes into the existing messaging subsystem.  
> **Focus:** High-level design, architecture, and integration points. No source code changes.

---

## 1. Design Goal

The catalogue should behave like a public marketplace storefront:

- **Anyone** can browse, search, and view catalogue entries.
- **Authenticated users** can take actions: message the owning party, save/follow an item, request a deal, report an item.
- **Owning parties** retain full control over their catalogue items and incoming inquiries.
- **Admins** continue to moderate public content.

This turns catalogue entries from internal deal attachments into **market-facing, searchable listings** that drive liquidity on the platform.

---

## 2. Guiding Principles

1. **Read path is public; write path is protected.** All `GET` catalogue operations work without a token. All mutations, admin operations, and action endpoints require a valid bearer token and appropriate scope.
2. **Anonymous responses are sanitized.** Public views omit private metadata, admin notes, exact coordinates, and direct owner contact information unless the owner has opted in.
3. **Reuse the existing messaging domain.** “Contact owner” is implemented as a new conversation context (`CATALOG_ITEM`) rather than a bespoke inquiry table. This gives the feature threads, read receipts, notifications, and moderation for free.
4. **Opt-in safety controls.** Parties can disable incoming catalogue messages, and the platform can rate-limit anonymous/authenticated contact attempts.
5. **No leakage of deal data.** Deal-bound copies remain private to deal participants.

---

## 3. Public Read Model

### 3.1 What anonymous users can see

The public catalogue response contains only non-sensitive fields:

- Item identity, name, description, category, and category path.
- Quantity, unit, condition (resources), priority (needs), skills/equipment (enhancements).
- Rough location: city/region/country from `location_address`, not exact lat/lng.
- Availability window.
- Platform trust signals: `verified_by_platform`, owner display name (not email), owner trust score tier, deal count.
- Whether the owner accepts catalogue inquiries.

### 3.2 What is hidden from anonymous users

- Exact `latitude` / `longitude`.
- `metadata` JSONB unless explicitly marked public.
- `opportunity_cost`, `max_budget`, `estimated_input_cost` unless the owner chooses to expose them.
- `admin_notes`, `admin_reviewed_by`, `platform_hidden`, `platform_featured` internals.
- Any deal-bound copy links.

### 3.3 Public search & filtering

All existing catalogue search filters remain available to anonymous callers:

- Text search across name/description.
- Category, domain, sub-category.
- Geo filter using city/region or approximate radius.
- Condition, availability window, quantity.
- Sort by newest, relevance, trust score, approximate distance.

Anonymous users receive the same paginated list shape as authenticated users, but each item is rendered through the public projection.

---

## 4. Authenticated Actions

### 4.1 Core action: contact the catalogue owner

Introduce a single, generic action endpoint:

```text
POST /api/v1/catalog/{itemType}/{itemId}/contact
```

where `itemType` is `resources`, `needs`, or `enhancements`.

Request body:

```json
{
  "message": "Hi, I'm interested in your 10-acre farmland listing. Is it still available for the 2026 season?",
  "acting_party_id": "..." // optional; used when user belongs to multiple parties
}
```

The platform:

1. Authenticates the caller.
2. Resolves the acting party (from `X-Party-ID` header or body, with fallback to sole membership).
3. Loads the catalogue item and verifies the owner accepts inquiries.
4. Creates or reuses a `Conversation` of type `CATALOG_ITEM` between the caller and the owner party.
5. Sends the initial message into that conversation.
6. Notifies the owner party members via the notification subsystem.

This reuses the existing `messages` domain (`Conversation`, `Message`, read receipts, notifications) without adding a new inquiry table.

### 4.2 Additional actions (optional, same auth model)

| Action | Endpoint | Notes |
|--------|----------|-------|
| Save / follow | `POST /api/v1/catalog/{type}/{id}/save` | Creates a user-saved item row for bookmarking |
| Request deal | `POST /api/v1/catalog/{type}/{id}/deal-request` | Starts a lightweight deal-proposal flow |
| Report item | `POST /api/v1/catalog/{type}/{id}/report` | Submits a moderation report tied to the item |

For the first phase, only **contact owner** is mandatory; the others can be stubbed or deferred.

---

## 5. Auth Middleware Design Options

The existing auth middleware whitelists exact `(method, path)` pairs. Catalogue routes have path parameters and many public read paths, so exact matching is impractical. Two high-level options exist.

Prefix-based public route matching (recommended)

Refactor the public-route check to support path-prefix wildcards in addition to exact matches:

```text
GET  /api/v1/catalog/*
GET  /api/v1/resources
GET  /api/v1/resources/*
GET  /api/v1/needs
GET  /api/v1/needs/*
GET  /api/v1/enhancements
GET  /api/v1/enhancements/*
GET  /api/v1/discovery/*
```

Rules:

- `GET` requests under these prefixes pass through without a token.
- Non-`GET` requests under the same prefixes still require authentication.
- The handler layer checks `AuthContext` presence to decide whether to show owner-only details or enable action buttons.

**Why this is recommended:** It keeps route ownership centralized in one middleware, avoids splitting route configuration, and makes the public intent explicit.

## 6. Conversation & Messaging Integration

### 6.1 New conversation context

Extend the `ConversationType` enum with a `CatalogItem` variant, or use the existing `Party`/`Deal` context with a `catalog_item_id` metadata field. A dedicated variant is preferred because it gives first-class semantics:

- System-generated subject line: `"Inquiry about: Irrigated Farmland - 10 Acres"`.
- Conversation participants: the inquirer (user) and the owner party.
- Reference: `catalog_item_id`, `catalog_item_type`.

### 6.2 Message flow

1. Caller invokes `POST /api/v1/catalog/resources/{id}/contact`.
2. Application use case `ContactCatalogOwner`:
   - Validates caller authentication.
   - Resolves acting party and role.
   - Verifies the catalogue item is public, active, and not hidden.
   - Checks owner has not disabled catalogue inquiries.
   - Looks for an existing `CATALOG_ITEM` conversation between caller and owner for this item; if none, creates one.
   - Inserts the initial message via `SendMessage` use case.
3. Notification worker sends an email/in-app notification to owner party members.
4. Owner replies through the normal message thread.

### 6.3 Safety & rate limiting

- **Owner opt-out:** Add a `accepts_catalog_inquiries` flag on `Party` or catalogue item metadata. If false, the contact endpoint returns `422` with a clear message.
- **Rate limiting:** Per-IP limit for anonymous contact attempts (not applicable here because contact requires auth), and per-user limits on total contact messages per hour to prevent spam.
- **Blocklist:** Parties can block specific users from contacting them; blocked attempts return `403`.

---

## 7. Data Model Additions

### 7.1 Party-level controls

Add to the `parties` table:

- `accepts_catalog_inquiries BOOLEAN NOT NULL DEFAULT true`
- `public_contact_email BOOLEAN NOT NULL DEFAULT false` — whether to expose the party email on catalogue pages.

These are owner-configurable via a new party setting endpoint (e.g., `PATCH /api/v1/parties/{id}/catalog-settings`).

### 7.2 Catalogue item visibility flags

No new table columns are required beyond the previous plan. The public projection uses:

- `is_active = true`
- `platform_hidden = false`
- owner party `is_active = true`

### 7.3 Saved items (optional)

If “save” is implemented:

```sql
CREATE TABLE saved_catalog_items (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_type TEXT NOT NULL CHECK (item_type IN ('RESOURCE','NEED','ENHANCEMENT')),
    item_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, item_type, item_id)
);
```

### 7.4 Deal-request bridge (optional)

If “request deal” is implemented, store a lightweight `deal_request` row that the owner can accept to seed a new deal draft.

---

## 8. API Endpoint Adjustments

### Public (no token required)

All `GET` endpoints return the public projection unless the caller is authenticated and authorized to see more.

| Method | Endpoint |
|--------|----------|
| GET | `/api/v1/resources` |
| GET | `/api/v1/resources/{id}` |
| GET | `/api/v1/resources/search` |
| GET | `/api/v1/resources/categories` |
| GET | `/api/v1/needs` |
| GET | `/api/v1/needs/{id}` |
| GET | `/api/v1/needs/categories` |
| GET | `/api/v1/enhancements` |
| GET | `/api/v1/enhancements/{id}` |
| GET | `/api/v1/enhancements/categories` |
| GET | `/api/v1/discovery/domains` |
| GET | `/api/v1/discovery/domains/{id}` |

### Authenticated actions

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/v1/catalog/resources/{id}/contact` | Message the supplier owner |
| POST | `/api/v1/catalog/needs/{id}/contact` | Message the consumer owner |
| POST | `/api/v1/catalog/enhancements/{id}/contact` | Message the enhancer owner |
| POST | `/api/v1/catalog/{type}/{id}/save` | Save item (optional) |
| POST | `/api/v1/catalog/{type}/{id}/report` | Report item (optional) |

### Owner/admin catalogue management (already protected)

| Method | Endpoint |
|--------|----------|
| POST/PUT/PATCH/DELETE | `/api/v1/resources`, `/api/v1/resources/{id}` |
| POST/PUT/PATCH/DELETE | `/api/v1/needs`, `/api/v1/needs/{id}` |
| POST/PUT/PATCH/DELETE | `/api/v1/enhancements`, `/api/v1/enhancements/{id}` |
| PATCH | `/api/v1/admin/catalog/.../flags` |

---

## 9. Use-Case Layer Design

### New application use cases

| Use case | Responsibility |
|----------|----------------|
| `ContactCatalogOwner` | Validates auth, resolves conversation, sends initial message, emits notification |
| `GetCatalogItemPublic` | Returns public projection; enriches with owner trust signals |
| `GetCatalogItemOwner` | Returns full details when caller is owner/admin |
| `ListCatalogItemsPublic` | Public search/list with sanitized projection |
| `UpdatePartyCatalogSettings` | Toggle `accepts_catalog_inquiries` and contact-email exposure |

`GetCatalogItem` and `ListCatalogItems` from the original plan are split into public and owner-aware variants. The handler chooses the appropriate use case based on whether `AuthContext` is present and whether the caller owns the item.

### DTO projection

The response DTO for public reads should be a separate struct from the owner/admin response:

- `CatalogPublicResponse` — anonymous/any-authenticated view.
- `CatalogOwnerResponse` — extends public response with exact location, costs, admin flags, metadata, etc.

Handlers return the richer response only when the caller is the owner or holds `admin:catalogue`.

---

## 10. Notification & Discovery Implications

### 10.1 Notifications

- New notification types: `CATALOG_INQUIRY_RECEIVED`, `CATALOG_ITEM_FEATURED`.
- The notification payload references the catalogue item and conversation so the owner can jump directly to the thread.

### 10.2 Discovery

Public discovery endpoints should include:

- Live counts of active, public items per domain/category.
- Trending items based on recent views/inquiries (optional; can be a simple view counter).
- Featured items surfaced by admin flags.

A lightweight `catalog_item_views` table or Redis counter can track anonymous and authenticated views for ranking, but it is not required for the first phase.

---

## 11. Security & Privacy Checklist

- [ ] Public endpoints never expose exact coordinates or owner email unless explicitly public.
- [ ] Public endpoints never expose deal-bound links.
- [ ] `platform_hidden` and `is_active = false` items are excluded from all public results.
- [ ] Contact endpoints require authentication; anonymous users receive `401`.
- [ ] Contact endpoints respect the owner's `accepts_catalog_inquiries` setting.
- [ ] Rate limiting prevents spam through the contact endpoint.
- [ ] Admin endpoints remain protected by `admin:*` or `admin:catalogue`.
- [ ] Blocked users cannot contact a party.

---

## 12. Testing Considerations

- Unit tests for the public-vs-owner projection logic.
- Application tests for `ContactCatalogOwner` covering existing conversation reuse, opt-out rejection, and blocklist.
- API tests:
  - Anonymous `GET` catalogue returns `200` with sanitized fields.
  - Anonymous `POST /contact` returns `401`.
  - Authenticated `POST /contact` creates a conversation and message.
  - Owner settings block contact attempts.
- Coverage target remains > 85 % for all new catalogue code.

---

## 13. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Exact-path auth middleware cannot express public prefixes | Implement prefix/wildcard matching (Option A) or split route scopes (Option B). |
| Exposing owner data to anonymous users | Strict public projection + explicit opt-in for contact details. |
| Spam/abuse via contact endpoint | Rate limiting, owner opt-out, blocklist, moderation reports. |
| Message threads become noisy | Use dedicated `CATALOG_ITEM` conversation type with clear subject line. |
| Search-index scraping | Implement per-IP rate limiting on public search endpoints. |

---

## 14. Recommended Implementation Order

1. **Auth middleware** — add prefix-based public route matching for catalogue `GET` paths.
2. **Public projection** — introduce `CatalogPublicResponse` and split get/list use cases into public/owner variants.
3. **Owner settings** — add `accepts_catalog_inquiries` to `parties` and an update endpoint.
4. **Contact owner** — add `CATALOG_ITEM` conversation type and `ContactCatalogOwner` use case.
5. **Notifications** — wire `CATALOG_INQUIRY_RECEIVED` notifications.
6. **Rate limiting & blocklist** — add defensive controls around contact actions.
7. **Tests** — cover public, authenticated, owner, and error paths to maintain > 85 % coverage.

---

## 15. Decision Required

The main architectural decision is how to make catalogue `GET` endpoints public:

- **Option A (recommended):** Extend the auth middleware with prefix/wildcard public-route matching.
- **Option B:** Split route registration into public and authenticated scopes.

Both work; Option A keeps the route surface unified and auditable.
