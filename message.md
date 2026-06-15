# Messaging Feature — Design Specification

> **Scope:** Design the in-app messaging subsystem for Hayaland. It covers direct user-to-user messages, party-to-party messages, party↔user messages, party-member group chats, deal participant chats, platform-wide chatrooms, admin broadcasts, read receipts, soft deletion, replies, likes/dislikes, real-time delivery, and encrypted storage.
>
> **Audience:** Backend engineers, API consumers, and frontend engineers implementing the messaging feature.
>
> **Based on:** `3partydeal.pdf` §3.22 (Message), §3.23 (Notification), the existing Hayaland Rust codebase (hexagonal architecture, Actix Web + sqlx + PostgreSQL), and the top-level design documents (`party-guide.md`, `deal-plan.md`, `trust-score.md`, etc.).
>
> **Status:** Design only — no source code changes are described as already applied.

---

## 1. Goals

1. **Unified messaging model.** One `messages` table supports every messaging context (user↔user, party↔party, party↔user, party members, deal participants, chatrooms, admin broadcasts).
2. **Actor clarity.** Every message is sent by an authenticated `User`; when the user is acting on behalf of a `Party`, the `sender_party_id` is recorded for audit and access control.
3. **Contextual visibility.** A message belongs to a context (direct, party-internal, deal, chatroom, or admin broadcast). Recipients can only see messages they are authorized to see.
4. **Real-time delivery.** New messages, edits, deletes, read receipts, and reactions are pushed to connected clients over WebSocket.
5. **Encrypted at rest.** Message bodies are encrypted with AES-256-GCM before being written to PostgreSQL; TLS protects data in transit.
6. **Soft delete & audit.** Deleted messages remain in the database as placeholders; only senders or admins can delete; only senders or admins can edit.
7. **Engagement primitives.** Messages support threaded replies and `LIKE`/`DISLIKE` reactions.
8. **Trust-score input.** Response time and read behavior feed the `response_rate` component of the trust-score calculator.

---

## 2. Out of Scope (Future Work)

- **Email/SMS/push notifications** — the existing email subsystem handles transactional email; in-app notifications may reference messages but outbound multi-channel alerts are a separate feature.
- **End-to-end encryption** — the platform retains the ability to decrypt message bodies for search, moderation, audit, and notifications.
- **Voice/video calls**, message editing history, message drafts, typing indicators, and message reactions beyond like/dislike.
- **Media uploads** — attachment URLs are stored, but object-storage upload endpoints are not part of this design.

---

## 3. Core Concepts

| Concept | Meaning |
|---|---|
| **User** | Authenticated account. Always the ultimate actor. |
| **Party** | Business identity. A user may belong to multiple parties; `X-Party-ID` selects the acting party. |
| **Conversation** | A thread that groups related messages (direct, party-internal, deal, admin broadcast). |
| **Message** | A single communication record with encrypted content, sender, context, and metadata. |
| **Read receipt** | A record that a specific user has read a specific message. |
| **Reaction** | A `LIKE` or `DISLIKE` attached to a message by a user. |
| **Admin broadcast** | A message sent by a platform admin to all parties and/or all users. |
| **ChatRoom** | A platform-wide room that any user or party can join and participate in. |
| **ChatRoomMembership** | A user's or party's membership in a chatroom. |

---

## 4. Messaging Contexts

The subsystem supports six messaging contexts through a single `recipient_type` discriminator.

| Context | `recipient_type` | Sender requirement | Recipient resolution |
|---|---|---|---|
| Direct user → user | `USER` | Any authenticated user | `recipient_user_id` |
| Party → party | `PARTY` | Member of sender party | `recipient_party_id` |
| Party ↔ user | `USER` or `PARTY` | Member of sender party (if sending as party) | `recipient_user_id` or `recipient_party_id` |
| Party members chat | `PARTY_MEMBERS` | Active member of the party | All active members of `sender_party_id` |
| Deal participant chat | `DEAL` | Member of a participating party, or admin | All parties in `recipient_deal_id` |
| Platform-wide chatroom | `ROOM` | Member of the chatroom | All members of `recipient_room_id` |
| Admin broadcast | `ADMIN_BROADCAST` | User with `admin:messages` or `admin:*` | All users and/or all parties |

A message always has:
- `sender_user_id` — the authenticated user who performed the send.
- `sender_party_id` — optional; present when the user is acting as a party.
- One of `recipient_user_id`, `recipient_party_id`, `recipient_deal_id`, `recipient_room_id`, or `recipient_type = ADMIN_BROADCAST`.

---

## 5. Domain Model

### 5.1 `Conversation`

A conversation is a lightweight grouping container. It is created lazily when the first message in a context is sent, except for chatrooms, where the conversation is created when the room is created.

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Primary key. |
| `conversation_type` | Enum | `DIRECT_USER`, `DIRECT_PARTY`, `PARTY_MEMBERS`, `DEAL`, `ROOM`, `ADMIN_BROADCAST`. A `ROOM` conversation is created when the chatroom is created. |
| `user_a_id` | UUID | For `DIRECT_USER`; one participant. |
| `user_b_id` | UUID | For `DIRECT_USER`; the other participant. |
| `party_a_id` | UUID | For `DIRECT_PARTY`; one party. |
| `party_b_id` | UUID | For `DIRECT_PARTY`; the other party. |
| `party_id` | UUID | For `PARTY_MEMBERS`; the party whose members chat. |
| `deal_id` | UUID | For `DEAL`; the deal context. |
| `room_id` | UUID | For `ROOM`; the chatroom context. |
| `title` | Text | Optional display title for group contexts. |
| `last_message_at` | TIMESTAMPTZ | Updated on every new message. |
| `created_at` | TIMESTAMPTZ | Auto-set. |

Uniqueness rules:
- `DIRECT_USER`: unique `(user_a_id, user_b_id)` with `user_a_id < user_b_id`.
- `DIRECT_PARTY`: unique `(party_a_id, party_b_id)` with `party_a_id < party_b_id`.
- `PARTY_MEMBERS`: unique `(party_id)`.
- `DEAL`: unique `(deal_id)`.
- `ADMIN_BROADCAST`: a single reserved broadcast conversation (or none; broadcasts are standalone).
- `ROOM`: unique `(room_id)`.

### 5.2 `ChatRoom`

A `ChatRoom` is a platform-wide channel that users and parties can join. Membership controls who can read and send messages in the room.

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Primary key. |
| `name` | Text | Unique, human-readable room name (3–120 chars). |
| `description` | Text | Optional public description. |
| `room_type` | Enum | `PUBLIC` (any active user/party can join) or `PRIVATE` (invite/membership required). |
| `created_by_user_id` | UUID | FK → `users`. The user who created the room. |
| `is_deleted` | Boolean | Soft-delete flag. Default `false`. |
| `created_at` | TIMESTAMPTZ | Auto-set. |
| `updated_at` | TIMESTAMPTZ | Auto-set. |

### 5.3 `ChatRoomMembership`

Links a user or party to a chatroom. A user can be a member directly, or they can participate on behalf of a party they belong to.

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Primary key. |
| `room_id` | UUID | FK → `chat_rooms`. |
| `user_id` | UUID | FK → `users`, nullable. The user member. |
| `party_id` | UUID | FK → `parties`, nullable. The party member. |
| `member_role` | Enum | `MEMBER` or `MODERATOR`. |
| `joined_at` | TIMESTAMPTZ | Auto-set. |
| UNIQUE(room_id, user_id) | | One user membership per room. |
| UNIQUE(room_id, party_id) | | One party membership per room. |

At least one of `user_id` or `party_id` must be non-null. A party membership grants every active member of that party access to the room.

### 5.4 `Message`

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Primary key. |
| `conversation_id` | UUID | FK → `conversations`. |
| `sender_user_id` | UUID | FK → `users`. The authenticated user. |
| `sender_party_id` | UUID | FK → `parties`, nullable. The party the user acted as. |
| `recipient_type` | Text | `USER`, `PARTY`, `PARTY_MEMBERS`, `DEAL`, `ROOM`, `ADMIN_BROADCAST`. |
| `recipient_user_id` | UUID | Nullable. |
| `recipient_party_id` | UUID | Nullable. |
| `recipient_deal_id` | UUID | Nullable. |
| `recipient_room_id` | UUID | Nullable. |
| `message_type` | Text | `TEXT`, `FILE`, `SYSTEM`, `ADMIN_BROADCAST`. |
| `subject` | Text | Optional subject line. |
| `content` | Text | AES-256-GCM ciphertext, base64-encoded (IV + tag + ciphertext). |
| `content_plaintext` | — | Never stored. Decrypted in memory on read. |
| `attachment_urls` | TEXT[] | Object-storage references. |
| `reply_to_message_id` | UUID | Nullable; threading reference. |
| `is_deleted` | Boolean | Soft-delete flag. Default `false`. |
| `edited_at` | TIMESTAMPTZ | Nullable; set when content is edited. |
| `created_at` | TIMESTAMPTZ | Auto-set. |

### 5.5 `MessageRead`

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Primary key. |
| `message_id` | UUID | FK → `messages`. |
| `user_id` | UUID | FK → `users`. The user who read it. |
| `party_id` | UUID | FK → `parties`, nullable. The party context if read as party. |
| `read_at` | TIMESTAMPTZ | Auto-set. |
| UNIQUE(message_id, user_id) | | One read record per user per message. |

### 5.6 `MessageReaction`

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Primary key. |
| `message_id` | UUID | FK → `messages`. |
| `user_id` | UUID | FK → `users`. |
| `party_id` | UUID | FK → `parties`, nullable. |
| `reaction_type` | Text | `LIKE` or `DISLIKE`. |
| `created_at` | TIMESTAMPTZ | Auto-set. |
| UNIQUE(message_id, user_id, party_id, reaction_type) | | One reaction of each type per user/party. |

---

## 6. Encryption Design

### 6.1 Storage format

Message bodies are encrypted before persistence:

```text
content = base64(IV || TAG || CIPHERTEXT)
```

- Algorithm: AES-256-GCM.
- IV: 12 bytes, randomly generated per message.
- Tag: 16 bytes, produced by GCM.
- Key: one platform data-encryption key (DEK) loaded from `APP_MESSAGES__ENCRYPTION_KEY` at startup.
- Key rotation: a `content_encryption_key_id` column references a key registry table (`encryption_keys`) so older messages can be decrypted with their original key.

### 6.2 Key management (MVP)

- A single DEK is stored in the application configuration/secrets system (e.g., `.env` or a secrets manager).
- The key is never stored in the database.
- Rotation: generate a new DEK, mark the old key as deprecated in `encryption_keys`, and decrypt/re-encrypt historical messages via a background task or on read.

### 6.3 Search & moderation trade-off

Because the platform holds the DEK, the application layer can decrypt content after reading it from Postgres. Full-text search can be implemented by:
- Decrypting messages in memory on search, or
- Maintaining a separate search index (e.g., PostgreSQL `tsvector` column updated when a message is created) if the decrypted content is written to the index at send time.

This trade-off is acceptable because the chosen encryption scope is **at-rest protection**, not end-to-end privacy.

---

## 7. Access Control & Authorization

### 7.1 Scopes

Add to `role_definitions` via a migration:

- `messages:read` — read messages the caller is authorized to see.
- `messages:write` — send, edit own messages, react, mark read.
- `chatrooms:read` — list and view chatrooms the caller can access.
- `chatrooms:write` — create, update, and soft-delete chatrooms.
- `chatrooms:moderate` — manage memberships and moderate messages in any chatroom.
- `admin:messages` — send admin broadcasts, edit/delete any message, manage any chatroom.

`admin:*` implicitly grants `admin:messages`.

### 7.2 Send rules

| Context | Who can send |
|---|---|
| Direct user → user | Any active authenticated user. |
| Party → party | Active member (`OWNER`, `ADMIN`, `MEMBER`) of sender party. |
| Party → user | Active member of sender party. |
| User → party | Any active user; recipient is the party (all members see it). |
| Party members chat | Active member of that party. |
| Deal chat | Active member of one of the three participating parties, or a user with `admin:deals`/`admin:*`. |
| Platform-wide chatroom | Active member of the chatroom. |
| Admin broadcast | User with `admin:messages` or `admin:*`. |

### 7.3 View rules

A user can read a message if any of the following is true:
- `recipient_type = USER` and `recipient_user_id = user.id`.
- `recipient_type = PARTY` and the user is an active member of `recipient_party_id`.
- `recipient_type = PARTY_MEMBERS` and the user is an active member of `sender_party_id`.
- `recipient_type = DEAL` and the user is an active member of a party participating in `recipient_deal_id`, or the user is an admin.
- `recipient_type = ROOM` and the user is an active member of the room (directly or through a party membership), or the user has `chatrooms:moderate`/`admin:messages`/`admin:*`.
- `recipient_type = ADMIN_BROADCAST` — all users can read (or a subset based on broadcast targeting).
- The user is the sender.
- The user has `admin:messages` or `admin:*`.

### 7.4 ChatRoom management rules

- **Create room**: user with `chatrooms:write`, `chatrooms:moderate`, `admin:messages`, or `admin:*`.
- **Update room metadata** (name, description, room_type): creator, room moderator, or admin.
- **Soft-delete room**: creator, room moderator, or admin. Deletion sets `chat_rooms.is_deleted = true` and blocks new messages; historical messages remain visible to members.
- **Manage memberships**: room creator, moderator, or admin can add/remove members and assign moderator role.
- **Join/leave public room**: any active user/party can join a `PUBLIC` room; members can leave. `PRIVATE` rooms require an invite or moderator action.

### 7.5 Edit rules

- Only the original `sender_user_id` or an admin can edit.
- `SYSTEM` and `ADMIN_BROADCAST` messages cannot be edited by non-admins.
- Editing updates `content` (re-encrypted) and sets `edited_at`.
- Reactions and read receipts are preserved.

### 7.6 Soft-delete rules

- Only the original `sender_user_id` or an admin can soft-delete.
- Deletion sets `is_deleted = true` and clears `content` (the ciphertext is overwritten with an empty encrypted placeholder).
- The row remains so threads keep their shape and read receipts/reactions remain valid.

### 7.7 Reaction rules

- Any user authorized to view the message can react.
- A user may have one `LIKE` and one `DISLIKE` per message; toggling removes the previous reaction of the same type.

---

## 8. Real-Time Delivery (WebSocket)

### 8.1 Connection model

- One WebSocket connection per authenticated user.
- The connection URL is `/api/v1/ws/messages`.
- On handshake, the client sends a JWT Bearer token in a query parameter or in the `Authorization` header; the server validates it and creates a session.

### 8.2 Channel subscription

After authentication, the server subscribes the connection to channels derived from the user's identity:

- `user:{user_id}` — direct user messages.
- `party:{party_id}` — for each party the user is an active member of.
- `deal:{deal_id}` — for each deal any of those parties participates in.
- `room:{room_id}` — for each chatroom the user or any of their parties is a member of.
- `admin:broadcast` — only if the user has `admin:messages` or `admin:*`.

### 8.3 Events pushed to clients

```json
{
  "event": "message.new",
  "payload": {
    "messageId": "...",
    "conversationId": "...",
    "senderUserId": "...",
    "senderPartyId": "...",
    "recipientType": "DEAL",
    "recipientDealId": "...",
    "messageType": "TEXT",
    "subject": null,
    "content": "plaintext content",
    "replyToMessageId": null,
    "isDeleted": false,
    "editedAt": null,
    "createdAt": "2026-06-15T10:00:00Z"
  }
}
```

Other events:
- `message.updated` — content edited.
- `message.deleted` — soft deleted.
- `message.read` — a user read a message.
- `message.reaction` — like/dislike added or removed.

### 8.4 Write path

Clients do **not** send messages over WebSocket. They use the REST API. The server persists the message, decrypts/encrypts as needed, and then publishes the event to the appropriate channels for delivery to connected clients.

### 8.5 Horizontal scaling

For a single-node MVP, the Actix Web server can keep an in-memory map of `user_id → WebSocket session` and publish directly.

For multi-node deployments, use a fan-out broker:
- **Redis Pub/Sub** — simplest; message event published to Redis, all API instances subscribe and forward to local sessions.
- **PostgreSQL LISTEN/NOTIFY** — avoids adding infrastructure; payload size limits apply.

The design document will recommend Redis Pub/Sub for production but allow the MVP to use in-memory fan-out.

---

## 9. API Contracts

All endpoints require `Authorization: Bearer <jwt>`. Endpoints where the caller acts as a party require `X-Party-ID`.

### 9.1 Send a message

```http
POST /api/v1/messages
Authorization: Bearer <jwt>
X-Party-ID: <party-id>   # optional; required when acting as a party
Content-Type: application/json

{
  "recipientType": "DEAL",
  "recipientDealId": "deal-uuid",
  "messageType": "TEXT",
  "subject": "Question about delivery",
  "content": "Can we move the delivery date to Friday?",
  "replyToMessageId": "message-uuid-optional",
  "attachmentUrls": []
}
```

Response: `201 Created` with the created message.

### 9.2 List conversations

```http
GET /api/v1/conversations?page=1&per_page=20
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Returns conversations where the user (or acting party) is a participant, ordered by `last_message_at DESC`.

### 9.3 List messages in a conversation

```http
GET /api/v1/conversations/{conversation_id}/messages?before_id=&limit=50
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Returns messages the caller is authorized to see, oldest first, with read/reaction summaries.

### 9.4 Get a single message

```http
GET /api/v1/messages/{message_id}
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

### 9.5 Edit a message

```http
PATCH /api/v1/messages/{message_id}
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json

{
  "content": "Updated text"
}
```

Allowed only for sender or admin. Returns the updated message.

### 9.6 Soft-delete a message

```http
DELETE /api/v1/messages/{message_id}
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Allowed only for sender or admin. Returns `204 No Content`.

### 9.7 Mark a message as read

```http
POST /api/v1/messages/{message_id}/read
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Idempotent. Returns `200 OK` with the read receipt.

### 9.8 React to a message

```http
POST /api/v1/messages/{message_id}/reactions
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json

{
  "reactionType": "LIKE"
}
```

Toggle behavior: if the user already has a `LIKE`, calling it again removes it.

### 9.9 Remove a reaction

```http
DELETE /api/v1/messages/{message_id}/reactions/LIKE
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

### 9.10 Global unread count

```http
GET /api/v1/messages/unread-count
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Returns the number of messages the user (or acting party) is authorized to see but has not read.

### 9.11 List chatrooms

```http
GET /api/v1/chatrooms?type=PUBLIC&page=1&per_page=20
Authorization: Bearer <jwt>
```

Returns chatrooms the caller can see. `PUBLIC` rooms are visible to everyone; `PRIVATE` rooms are visible only to members, moderators, and admins.

### 9.12 Create a chatroom

```http
POST /api/v1/chatrooms
Authorization: Bearer <jwt>
Content-Type: application/json

{
  "name": "Agriculture Deals",
  "description": "Discussion space for agriculture domain deals",
  "roomType": "PUBLIC"
}
```

Requires `chatrooms:write`, `chatrooms:moderate`, `admin:messages`, or `admin:*`.

### 9.13 Get a chatroom

```http
GET /api/v1/chatrooms/{room_id}
Authorization: Bearer <jwt>
```

Visible to members, moderators, and admins; `PUBLIC` rooms are also visible to any authenticated user.

### 9.14 Update a chatroom

```http
PATCH /api/v1/chatrooms/{room_id}
Authorization: Bearer <jwt>
Content-Type: application/json

{
  "name": "Agriculture Deals & News",
  "description": "Updated description"
}
```

Allowed for creator, moderator, or admin.

### 9.15 Soft-delete a chatroom

```http
DELETE /api/v1/chatrooms/{room_id}
Authorization: Bearer <jwt>
```

Allowed for creator, moderator, or admin. Returns `204 No Content`.

### 9.16 Join a chatroom

```http
POST /api/v1/chatrooms/{room_id}/members
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Joins the acting user (or party, if `X-Party-ID` is provided) to the room. `PUBLIC` rooms allow self-join; `PRIVATE` rooms require `chatrooms:moderate` or admin scope.

### 9.17 Leave a chatroom

```http
DELETE /api/v1/chatrooms/{room_id}/members/me
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Removes the caller's membership. Admins can remove any member via `DELETE /api/v1/chatrooms/{room_id}/members/{member_id}`.

### 9.18 List chatroom messages

```http
GET /api/v1/chatrooms/{room_id}/messages?before_id=&limit=50
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
```

Requires active membership (or moderator/admin access).

### 9.19 Send a message to a chatroom

```http
POST /api/v1/messages
Authorization: Bearer <jwt>
X-Party-ID: <party-id>
Content-Type: application/json

{
  "recipientType": "ROOM",
  "recipientRoomId": "room-uuid",
  "messageType": "TEXT",
  "content": "Hello everyone!"
}
```

Requires active membership in the room.

### 9.20 Admin broadcast

```http
POST /api/v1/admin/messages/broadcast
Authorization: Bearer <jwt>
Content-Type: application/json

{
  "target": "ALL_USERS_AND_PARTIES",
  "messageType": "ADMIN_BROADCAST",
  "subject": "Scheduled maintenance",
  "content": "The platform will be down for maintenance on Sunday 02:00 UTC."
}
```

`target` can be `ALL_USERS`, `ALL_PARTIES`, or `ALL_USERS_AND_PARTIES`.

---

## 10. Database Schema

All migrations are additive and idempotent.

```sql
-- Encryption key registry (for key rotation)
CREATE TABLE IF NOT EXISTS encryption_keys (
    id UUID PRIMARY KEY,
    key_name TEXT NOT NULL UNIQUE,
    key_bytes TEXT NOT NULL,  -- base64-encoded key material; loaded from config or KMS in MVP
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Platform-wide chatrooms.
CREATE TABLE IF NOT EXISTS chat_rooms (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    room_type TEXT NOT NULL CHECK (room_type IN ('PUBLIC','PRIVATE')),
    created_by_user_id UUID NOT NULL REFERENCES users(id),
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_chat_rooms_type
    ON chat_rooms(room_type) WHERE is_deleted = false;
CREATE INDEX IF NOT EXISTS idx_chat_rooms_deleted
    ON chat_rooms(is_deleted);

-- Chatroom memberships.
CREATE TABLE IF NOT EXISTS chat_room_memberships (
    id UUID PRIMARY KEY,
    room_id UUID NOT NULL REFERENCES chat_rooms(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    party_id UUID REFERENCES parties(id) ON DELETE CASCADE,
    member_role TEXT NOT NULL DEFAULT 'MEMBER' CHECK (member_role IN ('MEMBER','MODERATOR')),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (room_id, user_id),
    UNIQUE (room_id, party_id),
    CONSTRAINT chk_membership_actor CHECK (
        (user_id IS NOT NULL AND party_id IS NULL) OR
        (user_id IS NULL AND party_id IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_chat_room_memberships_room
    ON chat_room_memberships(room_id);
CREATE INDEX IF NOT EXISTS idx_chat_room_memberships_user
    ON chat_room_memberships(user_id);
CREATE INDEX IF NOT EXISTS idx_chat_room_memberships_party
    ON chat_room_memberships(party_id);

-- Conversations group messages by context.
CREATE TABLE IF NOT EXISTS conversations (
    id UUID PRIMARY KEY,
    conversation_type TEXT NOT NULL
        CHECK (conversation_type IN ('DIRECT_USER','DIRECT_PARTY','PARTY_MEMBERS','DEAL','ADMIN_BROADCAST')),
    user_a_id UUID REFERENCES users(id),
    user_b_id UUID REFERENCES users(id),
    party_a_id UUID REFERENCES parties(id),
    party_b_id UUID REFERENCES parties(id),
    party_id UUID REFERENCES parties(id),
    deal_id UUID REFERENCES deals(id),
    title TEXT,
    last_message_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_direct_user_members CHECK (
        conversation_type != 'DIRECT_USER' OR (user_a_id IS NOT NULL AND user_b_id IS NOT NULL)
    ),
    CONSTRAINT chk_direct_party_members CHECK (
        conversation_type != 'DIRECT_PARTY' OR (party_a_id IS NOT NULL AND party_b_id IS NOT NULL)
    ),
    CONSTRAINT chk_party_members_context CHECK (
        conversation_type != 'PARTY_MEMBERS' OR party_id IS NOT NULL
    ),
    CONSTRAINT chk_deal_context CHECK (
        conversation_type != 'DEAL' OR deal_id IS NOT NULL
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_direct_user
    ON conversations(user_a_id, user_b_id) WHERE conversation_type = 'DIRECT_USER';
CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_direct_party
    ON conversations(party_a_id, party_b_id) WHERE conversation_type = 'DIRECT_PARTY';
CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_party_members
    ON conversations(party_id) WHERE conversation_type = 'PARTY_MEMBERS';
CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_deal
    ON conversations(deal_id) WHERE conversation_type = 'DEAL';
CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_room
    ON conversations(room_id) WHERE conversation_type = 'ROOM';

-- Messages store encrypted content.
CREATE TABLE IF NOT EXISTS messages (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_user_id UUID NOT NULL REFERENCES users(id),
    sender_party_id UUID REFERENCES parties(id),
    recipient_type TEXT NOT NULL
        CHECK (recipient_type IN ('USER','PARTY','PARTY_MEMBERS','DEAL','ROOM','ADMIN_BROADCAST')),
    recipient_user_id UUID REFERENCES users(id),
    recipient_party_id UUID REFERENCES parties(id),
    recipient_deal_id UUID REFERENCES deals(id),
    recipient_room_id UUID REFERENCES chat_rooms(id),
    message_type TEXT NOT NULL
        CHECK (message_type IN ('TEXT','FILE','SYSTEM','ADMIN_BROADCAST')),
    subject TEXT,
    content TEXT NOT NULL,  -- base64(IV || TAG || CIPHERTEXT)
    content_encryption_key_id UUID REFERENCES encryption_keys(id),
    attachment_urls TEXT[] NOT NULL DEFAULT '{}',
    reply_to_message_id UUID REFERENCES messages(id),
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    edited_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_message_recipient CHECK (
        (recipient_type = 'USER' AND recipient_user_id IS NOT NULL) OR
        (recipient_type = 'PARTY' AND recipient_party_id IS NOT NULL) OR
        (recipient_type = 'PARTY_MEMBERS' AND sender_party_id IS NOT NULL) OR
        (recipient_type = 'DEAL' AND recipient_deal_id IS NOT NULL) OR
        (recipient_type = 'ROOM' AND recipient_room_id IS NOT NULL) OR
        (recipient_type = 'ADMIN_BROADCAST')
    )
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_created
    ON messages(conversation_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_sender_user
    ON messages(sender_user_id);
CREATE INDEX IF NOT EXISTS idx_messages_recipient_user
    ON messages(recipient_user_id) WHERE recipient_type = 'USER';
CREATE INDEX IF NOT EXISTS idx_messages_recipient_party
    ON messages(recipient_party_id) WHERE recipient_type = 'PARTY';
CREATE INDEX IF NOT EXISTS idx_messages_recipient_deal
    ON messages(recipient_deal_id) WHERE recipient_type = 'DEAL';
CREATE INDEX IF NOT EXISTS idx_messages_recipient_room
    ON messages(recipient_room_id) WHERE recipient_type = 'ROOM';
CREATE INDEX IF NOT EXISTS idx_messages_reply_to
    ON messages(reply_to_message_id);

-- Read receipts.
CREATE TABLE IF NOT EXISTS message_reads (
    id UUID PRIMARY KEY,
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    party_id UUID REFERENCES parties(id),
    read_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (message_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_message_reads_message
    ON message_reads(message_id);
CREATE INDEX IF NOT EXISTS idx_message_reads_user
    ON message_reads(user_id);

-- Reactions (likes/dislikes).
CREATE TABLE IF NOT EXISTS message_reactions (
    id UUID PRIMARY KEY,
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    party_id UUID REFERENCES parties(id),
    reaction_type TEXT NOT NULL CHECK (reaction_type IN ('LIKE','DISLIKE')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (message_id, user_id, party_id, reaction_type)
);

CREATE INDEX IF NOT EXISTS idx_message_reactions_message
    ON message_reactions(message_id);
```

---

## 11. Crate-by-Crate Additions

These are the recommended modules when the feature is implemented. This design does not create or modify them.

### 11.1 `crates/domain`

```text
domain/src/entities/
  ├── message.rs            # Message, MessageType, RecipientType
  ├── conversation.rs       # Conversation, ConversationType
  ├── chat_room.rs          # ChatRoom, ChatRoomType
  ├── chat_room_membership.rs # ChatRoomMembership, ChatRoomMemberRole
  └── message_reaction.rs   # MessageReaction, ReactionType

domain/src/repositories/
  ├── message_repository.rs # MessageRepository port
  └── chat_room_repository.rs # ChatRoomRepository port
```

New `DomainError` variants:
- `MessageNotFound`
- `ConversationNotFound`
- `ChatRoomNotFound`
- `ChatRoomAlreadyExists`
- `ChatRoomMembershipNotFound`
- `AlreadyChatRoomMember`
- `InvalidChatRoomName`
- `InvalidMessageContent`
- `InvalidRecipient`
- `CannotEditMessage`
- `CannotDeleteMessage`
- `CannotManageChatRoom`
- `ReplyNotInSameContext`
- `InvalidReactionType`

### 11.2 `crates/application`

```text
application/src/messages/
  ├── send_message.rs
  ├── edit_message.rs
  ├── soft_delete_message.rs
  ├── list_conversations.rs
  ├── list_messages.rs
  ├── mark_read.rs
  ├── toggle_reaction.rs
  ├── admin_broadcast.rs
  └── dto.rs

application/src/chatrooms/
  ├── create_chat_room.rs
  ├── update_chat_room.rs
  ├── soft_delete_chat_room.rs
  ├── get_chat_room.rs
  ├── list_chat_rooms.rs
  ├── join_chat_room.rs
  ├── leave_chat_room.rs
  ├── manage_chat_room_membership.rs
  └── dto.rs
```

Outbound port:
- `RealtimePublisher: Send + Sync` with `publish(event: MessageEvent) -> Result<(), ApplicationError>`.
- `TrustScoreRecalculationPort` is reused to trigger recalculation when response behavior changes.

### 11.3 `crates/infrastructure`

```text
infrastructure/src/repositories/
  ├── postgres_message_repository.rs  # sqlx queries + AES-256-GCM encryption/decryption
  └── postgres_chat_room_repository.rs

infrastructure/src/realtime/
  └── in_memory_publisher.rs          # or redis_publisher.rs
```

### 11.4 `crates/api`

```text
api/src/routes/
  ├── messages.rs
  └── chatrooms.rs

api/src/handlers/messages/
  ├── send_message.rs
  ├── edit_message.rs
  ├── delete_message.rs
  ├── list_conversations.rs
  ├── list_messages.rs
  ├── mark_read.rs
  ├── react.rs
  ├── unread_count.rs
  └── admin_broadcast.rs

api/src/handlers/chatrooms/
  ├── create_chat_room.rs
  ├── update_chat_room.rs
  ├── delete_chat_room.rs
  ├── get_chat_room.rs
  ├── list_chat_rooms.rs
  ├── join_chat_room.rs
  ├── leave_chat_room.rs
  └── manage_membership.rs

api/src/websocket/
  └── message_socket.rs
```

---

## 12. Use Case Specifications

### 12.1 `SendMessage`

**Input:** `SendMessageCommand` with sender info, recipient type/IDs, content, optional reply target.

**Steps:**
1. Validate recipient type/ID combination.
2. Resolve or create the appropriate `Conversation`.
3. Validate sender authorization for the context.
4. If `reply_to_message_id` is provided, verify it exists in the same conversation.
5. Encrypt `content` with AES-256-GCM.
6. Insert `messages` row.
7. Update `conversations.last_message_at`.
8. Publish `message.new` event to the relevant channels.
9. Return `MessageResult`.

### 12.2 `EditMessage`

**Input:** `EditMessageCommand` with message ID, new content, actor info.

**Steps:**
1. Load message; return `MessageNotFound` if missing.
2. Verify actor is original sender or admin.
3. Verify message is not deleted.
4. Re-encrypt content; set `edited_at`.
5. Update row.
6. Publish `message.updated` event.

### 12.3 `SoftDeleteMessage`

**Input:** `SoftDeleteMessageCommand`.

**Steps:**
1. Load message.
2. Verify actor is original sender or admin.
3. Overwrite `content` with encrypted empty placeholder; set `is_deleted = true`.
4. Publish `message.deleted` event.

### 12.4 `MarkRead`

**Input:** `MarkReadCommand`.

**Steps:**
1. Load message; verify viewer authorization.
2. Insert `message_reads` row on conflict do nothing.
3. Publish `message.read` event.

### 12.5 `ToggleReaction`

**Input:** `ToggleReactionCommand`.

**Steps:**
1. Load message; verify viewer authorization.
2. If reaction exists for (message, user, party, type), delete it.
3. Otherwise insert it.
4. Publish `message.reaction` event with current totals.

### 12.6 `AdminBroadcast`

**Input:** `AdminBroadcastCommand`.

**Steps:**
1. Verify actor has `admin:messages` or `admin:*`.
2. Determine target audience (all users, all parties, or both).
3. Create one `messages` row per recipient (or per party, depending on target), all linked to the reserved admin broadcast conversation.
4. Publish `message.new` events to all affected channels.

> **Performance note:** for large platforms, broadcasting can be batched or performed via a background worker; the schema supports one row per recipient so read receipts work individually.

### 12.7 `CreateChatRoom`

**Input:** `CreateChatRoomCommand` with name, description, room type, creator user ID.

**Steps:**
1. Validate name (unique, 3–120 chars) and room type.
2. Verify actor has `chatrooms:write`, `chatrooms:moderate`, `admin:messages`, or `admin:*`.
3. Insert `chat_rooms` row.
4. Create the corresponding `conversations` row of type `ROOM`.
5. Add creator as a `MODERATOR` member.
6. Return `ChatRoomResult`.

### 12.8 `UpdateChatRoom`

**Input:** `UpdateChatRoomCommand`.

**Steps:**
1. Load room; return `ChatRoomNotFound` if missing or deleted.
2. Verify actor is creator, moderator, or admin.
3. Apply allowed updates (name, description, room_type).
4. Update `updated_at`.
5. Return updated `ChatRoomResult`.

### 12.9 `SoftDeleteChatRoom`

**Input:** `SoftDeleteChatRoomCommand`.

**Steps:**
1. Load room.
2. Verify actor is creator, moderator, or admin.
3. Set `is_deleted = true`; update `updated_at`.
4. Publish a `room.deleted` event so clients remove it from active lists.

### 12.10 `JoinChatRoom`

**Input:** `JoinChatRoomCommand` with room ID, user/party IDs.

**Steps:**
1. Load room; reject if deleted or private without permission.
2. For `PUBLIC` rooms: any active user/party can join; insert membership.
3. For `PRIVATE` rooms: require `chatrooms:moderate`, `admin:messages`, or `admin:*`.
4. Return membership result.

### 12.11 `LeaveChatRoom`

**Input:** `LeaveChatRoomCommand`.

**Steps:**
1. Load membership; verify it belongs to the actor.
2. Delete membership row.
3. If the last moderator leaves, the room remains but no new moderator is assigned automatically (admin intervention required).

### 12.12 `ManageChatRoomMembership`

**Input:** `ManageChatRoomMembershipCommand`.

**Steps:**
1. Verify actor is creator, moderator, or admin.
2. Add, remove, or update role of a target user/party in the room.
3. Prevent removing the creator unless by an admin.

---

## 13. Error Mapping

| Situation | Domain / Application Error | HTTP Status |
|---|---|---|
| Message not found | `MessageNotFound` / `NotFound` | 404 |
| Conversation not found | `ConversationNotFound` / `NotFound` | 404 |
| Chat room not found | `ChatRoomNotFound` / `NotFound` | 404 |
| Chat room name already exists | `ChatRoomAlreadyExists` / `Validation` | 409 |
| Chat room membership not found | `ChatRoomMembershipNotFound` / `NotFound` | 404 |
| Already a member of the chat room | `AlreadyChatRoomMember` / `Validation` | 409 |
| User not authorized to manage room | `CannotManageChatRoom` / `Forbidden` | 403 |
| Invalid recipient type/ID | `InvalidRecipient` / `Validation` | 422 |
| User not member of sender party | `Forbidden` | 403 |
| User not authorized to view context | `DealAccessDenied` / `Forbidden` | 403 |
| Actor is not sender or admin | `CannotEditMessage` / `CannotDeleteMessage` → `Forbidden` | 403 |
| Reply target in different conversation | `ReplyNotInSameContext` / `Validation` | 422 |
| Message already deleted | `Validation` | 422 |
| Invalid reaction type | `InvalidReactionType` / `Validation` | 422 |
| Admin scope missing | `Forbidden` | 403 |

---

## 14. Integration with Other Subsystems

### 14.1 Notifications

The messaging subsystem does not send email. It may emit in-app notifications via a future `Notification` feature. For now, WebSocket events are the real-time notification mechanism.

### 14.2 Trust Score

`SendMessage` and `MarkRead` provide data for the `response_rate` trust-score component:
- `average_response_hours` can be computed from timestamps of incoming messages and the user's replies.
- `response_rate` can be computed from read receipts.

Use the existing `TrustScoreRecalculationPort` to request recalculation for the sender's party after significant messaging activity.

### 14.3 Deal Lifecycle

Deal chat is available as soon as a deal exists, but it is most useful during `NEGOTIATING` and `EXECUTING`. The messaging subsystem does not gate sending on deal status except where access control requires active participation.

---

## 15. Testing Strategy

- **Domain tests**: recipient validation, edit/delete authorization, reaction rules.
- **Application tests** with fake repositories: send in each context, unauthorized send rejected, edit/delete by non-owner rejected, replies constrained to same conversation, read receipts, reactions.
- **Chatroom tests**: create/update/delete room authorization, public/private join rules, membership management, send/read messages in rooms.
- **Infrastructure tests**: Postgres `MessageRepository` and `ChatRoomRepository` CRUD; encryption round-trip; unique constraint enforcement; unread count query.
- **API tests**: all endpoints return correct status codes; missing `X-Party-ID` handled; WebSocket connect and event delivery.
- **CI**: keep `cargo fmt --check && cargo clippy -- -D warnings`, `cargo test`, and `cargo sqlx prepare --workspace --check` passing.

---

## 16. Migration Order

When implementing this design, create migrations in this order:

1. `seed_message_scopes.sql` — add `messages:read`, `messages:write`, `chatrooms:read`, `chatrooms:write`, `chatrooms:moderate`, `admin:messages` to `role_definitions`.
2. `create_encryption_keys.sql` — optional key registry table.
3. `create_chat_rooms.sql` — platform-wide chatrooms.
4. `create_chat_room_memberships.sql` — chatroom membership table.
5. `create_conversations.sql` — conversation grouping table.
6. `create_messages.sql` — encrypted message table.
7. `create_message_reads.sql` — read receipts.
8. `create_message_reactions.sql` — likes/dislikes.
9. `add_message_indexes.sql` — performance indexes.

All migrations must be idempotent (`CREATE TABLE IF NOT EXISTS`, `ADD COLUMN IF NOT EXISTS`, `CREATE INDEX IF NOT EXISTS`).

---

## 17. Security Considerations

1. **Encryption key protection.** The AES-256-GCM key must be loaded from a secret manager or environment variable; never committed to the repository.
2. **No plaintext logs.** Log only message IDs and metadata; never log decrypted content.
3. **Authorization at every layer.** Handlers check scopes; use cases verify context membership (including chatroom membership for `recipient_type = ROOM`); repositories enforce no direct unauthorized reads.
4. **Soft deletes only.** Hard deletion of messages is an admin-only, audited operation.
5. **Attachment security.** Attachment URLs should be presigned and time-limited; do not expose raw object-storage keys.
6. **WebSocket authentication.** Validate JWT on connect and revalidate periodically; reject unauthenticated connections immediately.

---

## 18. Open Points & Future Extensions

- **Push/email notifications** for unread messages when the user is offline.
- **Message search** via a dedicated search index or decrypted in-memory filtering.
- **Media uploads** with virus scanning and thumbnail generation.
- **Typing indicators** and **message delivery receipts** (separate from read receipts).
- **End-to-end encryption** as an optional mode for high-sensitivity contexts.
- **Chatroom moderation** tooling (kick/ban members, message retention policies, room audit logs).
- **Message moderation** tooling for admins (flag, hide, audit).

---

## 19. Glossary

| Term | Meaning |
|---|---|
| **Conversation** | A thread grouping messages by context (direct, party, deal, room, broadcast). |
| **Message** | A single encrypted communication record. |
| **Read receipt** | A record that a user has seen a message. |
| **Reaction** | A `LIKE` or `DISLIKE` attached to a message. |
| **Admin broadcast** | A message sent by a platform admin to a broad audience. |
| **ChatRoom** | A platform-wide channel that users and parties can join and participate in. |
| **ChatRoomMembership** | A record linking a user or party to a chatroom with a role. |
| **DEK** | Data-encryption key used for AES-256-GCM. |
| **Soft delete** | A message is marked deleted but remains in the database as a placeholder. |
