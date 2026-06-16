# Hayaland Notification Service — Implementation Design

> **Scope:** Design a complete, multi-channel notification subsystem for the existing Hayaland codebase that supports in-app, email, and push delivery; admin-initiated broadcasts; event-driven notifications; and scoped management.  
> **Constraint:** This document is a design and implementation plan. It does **not** modify source code.  
> **Coverage target:** ≥ 85 % line coverage for all new notification code.

---

## 1. Goals & Non-Goals

### 1.1 Goals
- Deliver **in-app**, **email**, and **push** notifications to users and parties.
- Allow **admins** (or users with the right scope) to send notifications to users, parties, or platform-wide.
- Emit notifications automatically from **domain lifecycle events**: deals, terms, milestones, payments, disputes, reviews, verifications, trust-score changes, and messages.
- Provide a user-facing **notification center** with read/unread/actioned states, filtering, and preferences.
- Support **quiet hours**, per-channel opt-outs, and per-notification-type channel preferences.
- Add **notification templates** for consistent, localisable messaging.
- Preserve the existing **hexagonal/clean architecture** and repository-port pattern.
- Re-use the existing background-worker, email-queue, and WebSocket-publisher infrastructure where possible.

### 1.2 Non-Goals
- SMS gateway integration in MVP (schema reserves the channel; provider is stubbed).
- Real mobile push provider (FCM/APNs) in MVP — provider port is stubbed with a no-op/recordable adapter.
- Distributed event bus (Kafka/RabbitMQ). The monolith emits notification commands synchronously through application use cases; an internal event envelope is introduced but backed by direct use-case calls.
- Webhook delivery to external clients in MVP (reserved in the `NotificationChannel` enum for future use).

---

## 2. Design Principles

1. **Fit the existing architecture.** New code lives in `crates/domain`, `crates/application`, `crates/infrastructure`, and `crates/api` exactly like `messages` and `email`.
2. **Ports first.** All delivery providers (email, push, in-app real-time) are hidden behind application ports so tests can use fakes.
3. **Persistence owns state.** The `notifications` table is the source of truth for in-app delivery, read status, actioned status, and delivery receipts.
4. **No polling required.** In-app notifications are pushed to connected clients via the existing WebSocket channel; clients can still list historical notifications via REST.
5. **Opt-out by default for intrusive channels.** Email/push require explicit user opt-in except for security/critical events.
6. **Admin control is scope-gated.** Sending platform notifications and managing templates require `admin:notifications` or `admin:*`.

---

## 3. Domain Model

### 3.1 Core Entity: `Notification`

New file: `crates/domain/src/entities/notification.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationChannel {
    InApp,
    Email,
    Push,
    Sms,        // reserved
    Webhook,    // reserved
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    // Deal lifecycle
    DealInvite,
    DealSubmitted,
    DealTermsLocked,
    DealCommitted,
    DealExecuting,
    DealCompleted,
    DealCancelled,
    DealExpired,
    DealDisputed,
    // Negotiation
    TermProposed,
    TermAccepted,
    TermRejected,
    TermCountered,
    // Milestones
    MilestoneAssigned,
    MilestoneStarted,
    MilestoneCompleted,
    MilestoneVerified,
    MilestoneDue,
    // Payments
    EscrowFunded,
    EscrowReleased,
    PaymentDue,
    PaymentReceived,
    TransactionPendingApproval,
    TransactionApproved,
    TransactionRejected,
    // Reviews / trust / disputes
    ReviewRequested,
    ReviewReceived,
    TrustScoreUpdated,
    DisputeOpened,
    DisputeResolved,
    VerificationApproved,
    VerificationRejected,
    // Messaging
    MessageReceived,
    Mentioned,
    // Admin / system
    AdminBroadcast,
    SystemMaintenance,
    SecurityAlert,
    Custom, // admin-initiated with explicit title/body
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Option<Uuid>,        // direct user recipient
    pub party_id: Option<Uuid>,        // party-level recipient (all members)
    pub notification_type: NotificationType,
    pub title: String,
    pub body: String,
    pub channels: Vec<NotificationChannel>, // channels this notification was routed to
    pub priority: NotificationPriority,
    pub status: NotificationStatus,
    pub read_at: Option<OffsetDateTime>,
    pub actioned_at: Option<OffsetDateTime>,
    pub expires_at: Option<OffsetDateTime>,
    pub action_url: Option<String>,
    pub actions: Vec<NotificationAction>,
    pub related_entity_type: Option<String>, // "deal", "milestone", "transaction", ...
    pub related_entity_id: Option<Uuid>,
    pub metadata: serde_json::Value,   // template variables, deep links, etc.
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationStatus {
    Pending,
    Sent,       // at least one channel succeeded
    Delivered,  // in-app read or email/push confirmed
    Failed,
    Suppressed, // by preference/quiet hours
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub label: String,
    pub action_type: ActionType, // Navigate, ApiCall, Dismiss
    pub url: Option<String>,
    pub method: Option<String>,
}
```

### 3.2 Entity: `NotificationPreference`

New file: `crates/domain/src/entities/notification_preference.rs`

```rust
#[derive(Debug, Clone)]
pub struct NotificationPreference {
    pub user_id: Uuid,
    pub channels: ChannelPreferences,
    pub per_type: HashMap<NotificationType, TypePreference>,
    pub quiet_hours: QuietHours,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct ChannelPreferences {
    pub in_app: bool,
    pub email: bool,
    pub push: bool,
    pub sms: bool,
}

#[derive(Debug, Clone)]
pub struct TypePreference {
    pub enabled: bool,
    pub channels: Vec<NotificationChannel>,
}

#[derive(Debug, Clone)]
pub struct QuietHours {
    pub enabled: bool,
    pub start: String, // "22:00"
    pub end: String,   // "07:00"
    pub timezone: String,
    pub except_critical: bool,
}
```

### 3.3 Entity: `NotificationTemplate`

New file: `crates/domain/src/entities/notification_template.rs`

```rust
#[derive(Debug, Clone)]
pub struct NotificationTemplate {
    pub id: Uuid,
    pub name: String,                    // "deal_invite_email"
    pub notification_type: NotificationType,
    pub channel: NotificationChannel,
    pub locale: String,                  // "en", "sw", etc.
    pub subject_template: String,
    pub body_template: String,
    pub variables_schema: serde_json::Value,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

### 3.4 Delivery Record

A separate `notification_delivery_records` table tracks per-channel attempts/success/failure without bloating the main notification row.

---

## 4. Repository Ports

New file: `crates/domain/src/repositories/notification_repository.rs`

```rust
#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn create(&self, notification: &Notification) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, DomainError>;
    async fn list_for_recipient(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
        filters: NotificationFilters,
        pagination: Pagination,
    ) -> Result<NotificationListResult, DomainError>;
    async fn count_unread_for_recipient(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
    ) -> Result<i64, DomainError>;
    async fn mark_read(&self, id: Uuid, user_id: Uuid, party_id: Option<Uuid>) -> Result<bool, DomainError>;
    async fn mark_all_read(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
        before: Option<OffsetDateTime>,
        notification_type: Option<NotificationType>,
    ) -> Result<u64, DomainError>;
    async fn mark_actioned(&self, id: Uuid, user_id: Uuid, party_id: Option<Uuid>) -> Result<bool, DomainError>;
    async fn delete(&self, id: Uuid, user_id: Uuid, party_id: Option<Uuid>) -> Result<bool, DomainError>;
    async fn update_status(&self, id: Uuid, status: NotificationStatus) -> Result<(), DomainError>;
    async fn record_delivery(
        &self,
        notification_id: Uuid,
        channel: NotificationChannel,
        result: DeliveryResult,
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait NotificationPreferenceRepository: Send + Sync {
    async fn get(&self, user_id: Uuid) -> Result<NotificationPreference, DomainError>;
    async fn save(&self, preference: &NotificationPreference) -> Result<(), DomainError>;
}

#[async_trait]
pub trait NotificationTemplateRepository: Send + Sync {
    async fn create(&self, template: &NotificationTemplate) -> Result<(), DomainError>;
    async fn update(&self, template: &NotificationTemplate) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<NotificationTemplate>, DomainError>;
    async fn find_active(
        &self,
        notification_type: NotificationType,
        channel: NotificationChannel,
        locale: &str,
    ) -> Result<Option<NotificationTemplate>, DomainError>;
    async fn list(&self, pagination: Pagination) -> Result<Vec<NotificationTemplate>, DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}
```

---

## 5. Application Layer

### 5.1 Outbound Ports (delivery adapters)

Extend `crates/application/src/ports.rs`:

```rust
// Existing MessageEvent stays unchanged; add a parallel NotificationEvent.
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    NotificationNew { notification_id: Uuid, user_id: Option<Uuid>, party_id: Option<Uuid> },
    NotificationRead { notification_id: Uuid, user_id: Uuid },
    UnreadCountChanged { user_id: Option<Uuid>, party_id: Option<Uuid>, count: i64 },
}

#[async_trait]
pub trait NotificationRealtimePublisher: Send + Sync {
    async fn publish(&self, event: NotificationEvent) -> Result<(), ApplicationError>;
}

#[async_trait]
pub trait PushNotificationSender: Send + Sync {
    async fn send(
        &self,
        device_tokens: &[String],
        title: &str,
        body: &str,
        data: serde_json::Value,
    ) -> Result<Vec<PushResult>, ApplicationError>;
}

#[async_trait]
pub trait SmsSender: Send + Sync {
    async fn send(&self, phone: &str, body: &str) -> Result<(), ApplicationError>;
}
```

### 5.2 Use Cases

Directory: `crates/application/src/notifications/`

| Use Case | Responsibility |
|----------|----------------|
| `CreateNotification` | Low-level creation used by event triggers. Persists row, routes to channels, publishes real-time event. |
| `SendNotification` | Internal orchestrator: resolve recipients, render templates, apply preferences/quiet hours, persist, dispatch. |
| `ListNotifications` | Query notification center with filters (read, type, priority). |
| `GetNotification` | Single notification with ownership check. |
| `MarkNotificationRead` | Update `read_at`; publish `UnreadCountChanged`. |
| `MarkAllNotificationsRead` | Bulk update with optional type/date filters. |
| `MarkNotificationActioned` | Update `actioned_at`. |
| `DeleteNotification` | Soft or hard delete owned by recipient. |
| `GetUnreadCount` | Return `unread_count` for current user/party. |
| `GetNotificationPreferences` | Return preferences for user. |
| `UpdateNotificationPreferences` | Validate and save preferences. |
| `AdminSendNotification` | Admins send direct/custom notifications to users, parties, all users, or all parties. |
| `AdminListTemplates` / `AdminCreateTemplate` / `AdminUpdateTemplate` / `AdminDeleteTemplate` | Template CRUD. |
| `RenderNotification` | Internal helper — template resolution + variable substitution. |
| `RouteNotification` | Internal helper — applies priority, preferences, quiet hours to select channels. |

### 5.3 DTOs

`crates/application/src/notifications/dto.rs`:

```rust
pub struct SendNotificationCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub recipient: RecipientSelector,
    pub notification_type: NotificationType,
    pub priority: NotificationPriority,
    pub title: Option<String>,          // for CUSTOM only; else template renders
    pub body: Option<String>,
    pub action_url: Option<String>,
    pub actions: Vec<NotificationAction>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub locale: String,
}

pub enum RecipientSelector {
    User(Uuid),
    Party(Uuid),
    AllUsers,
    AllParties,
    PartyMembers { party_id: Uuid },
    DealParticipants { deal_id: Uuid },
}

pub struct NotificationResult { /* serialisable view */ }
pub struct NotificationListResult { pub items: Vec<NotificationResult>, pub unread_count: i64, pub total: i64 }
pub struct UnreadCountResult { pub count: i64 }
```

### 5.4 Routing Rules (from PDF §5.5 / §7.7)

Implemented in `RouteNotification`:

| Priority | Default channels |
|----------|------------------|
| Critical | in-app, push, email, SMS (if opted in) |
| High | in-app, push, email |
| Normal | in-app, email (if enabled) |
| Low | in-app, email digest (future) |

After priority defaults, filter by:
1. Per-type preference `enabled` flag.
2. Per-type `channels` list.
3. Global channel preferences.
4. Quiet hours (skip email/push/SMS unless `except_critical` and priority is Critical).
5. Device token availability for push.
6. Phone availability for SMS.

If all channels are filtered out, the notification is persisted with `status = Suppressed`.

---

## 6. Infrastructure Layer

### 6.1 PostgreSQL Repositories

New files:
- `crates/infrastructure/src/repositories/postgres_notification_repository.rs`
- `crates/infrastructure/src/repositories/postgres_notification_preference_repository.rs`
- `crates/infrastructure/src/repositories/postgres_notification_template_repository.rs`

Patterns follow `PostgresMessageRepository` — sqlx query_as! with offline metadata, `TryFrom` for domain enums using text storage, and idempotent `INSERT`/`UPDATE`.

### 6.2 Delivery Adapters

| Channel | Adapter file | Implementation |
|---------|--------------|----------------|
| Email | Re-use `crates/infrastructure/src/email/smtp_email_sender.rs` via existing `EmailQueue`. | Notification use case enqueues a rendered `EmailQueueItem`. |
| In-app real-time | `crates/infrastructure/src/realtime/notification_publisher.rs` | Wraps existing `SessionRegistry` to push `NotificationEvent` over WebSocket. |
| Push | `crates/infrastructure/src/notifications/fcm_push_sender.rs` | Stub that logs/records; production swaps in real FCM/APNs provider. |
| SMS | `crates/infrastructure/src/notifications/twilio_sms_sender.rs` | Stub; schema reserved. |

### 6.3 Notification Worker

New file: `crates/infrastructure/src/workers/notification_worker.rs`

A `tokio::spawn` loop (like `email_worker`) that:
1. Polls `notifications` table for rows in `Pending` status with channels not yet attempted.
2. Dispatches to email/push/SMS adapters.
3. Records delivery results in `notification_delivery_records`.
4. Updates `status` to `Sent`, `Delivered`, or `Failed`.
5. Retries failed channels with exponential backoff up to `max_retries`.

Configuration added to `EmailSettings` or a new `NotificationSettings` block in `config.rs`:

```rust
pub struct NotificationSettings {
    pub worker_enabled: bool,
    pub worker_interval_seconds: u64,
    pub worker_batch_size: usize,
    pub push_max_retries: u32,
    pub push_retry_base_delay_ms: u64,
    pub sms_max_retries: u32,
    pub default_locale: String,
}
```

---

## 7. API Layer

### 7.1 Routes

New file: `crates/api/src/routes/notifications.rs`

```rust
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/notifications")
            .route(web::get().to(handlers::notifications::list_notifications))
            .route(web::post().to(handlers::notifications::create_admin_notification)),
    )
    .service(
        web::resource("/notifications/unread-count")
            .route(web::get().to(handlers::notifications::unread_count)),
    )
    .service(
        web::resource("/notifications/actions/mark-all-read")
            .route(web::post().to(handlers::notifications::mark_all_read)),
    )
    .service(
        web::resource("/notifications/{id}")
            .route(web::get().to(handlers::notifications::get_notification))
            .route(web::patch().to(handlers::notifications::update_notification))
            .route(web::delete().to(handlers::notifications::delete_notification)),
    )
    .service(
        web::resource("/notifications/preferences")
            .route(web::get().to(handlers::notifications::get_preferences))
            .route(web::put().to(handlers::notifications::update_preferences)),
    )
    .service(
        web::resource("/admin/notifications/send")
            .route(web::post().to(handlers::notifications::admin_send)),
    )
    .service(
        web::resource("/admin/notification-templates")
            .route(web::get().to(handlers::notifications::admin_list_templates))
            .route(web::post().to(handlers::notifications::admin_create_template)),
    )
    .service(
        web::resource("/admin/notification-templates/{id}")
            .route(web::get().to(handlers::notifications::admin_get_template))
            .route(web::put().to(handlers::notifications::admin_update_template))
            .route(web::delete().to(handlers::notifications::admin_delete_template)),
    );
}
```

Mount under `/api/v1/` in `crates/api/src/routes/mod.rs`.

### 7.2 Handlers

New directory: `crates/api/src/handlers/notifications/`

Follow the thin-handler pattern of `crates/api/src/handlers/messages/send_message.rs`:
- Extract `AuthContext` and `X-Party-ID`.
- Validate JSON body with `validator`.
- Call use case.
- Map `ApplicationError` to `ApiError`.

Scope checks:
- `notifications:read` for listing/getting.
- `notifications:write` for marking read/actioned/deleting own notifications.
- `admin:notifications` or `admin:*` for admin send and template management.

### 7.3 WebSocket Event Extension

Extend `crates/api/src/websocket/message_socket.rs` `WsEvent` enum with a `NotificationNew` variant. The `WebSocketPublisher` implements `NotificationRealtimePublisher` and forwards `NotificationEvent` to the registry so connected clients receive live notification badges without polling.

---

## 8. Lifecycle Event Integration Points

Notifications are **not** emitted from controllers; they are emitted from existing application use cases after the business transaction commits. Each integration is a small, explicit call to `SendNotification` (or `CreateNotification`) at the end of the use case.

### 8.1 Deal Lifecycle

| Trigger location | Event | Recipients | Type / Priority |
|------------------|-------|------------|-----------------|
| `SubmitDeal::execute` → after state → `SUGGESTED` | Deal submitted | Other participating parties | `DealSubmitted` / Normal |
| `ExecuteTransition` → `TERMS_LOCKED` | Terms locked | All deal parties | `DealTermsLocked` / High |
| `ExecuteTransition` → `COMMITTED` | Deal committed | All deal parties | `DealCommitted` / High |
| `ExecuteTransition` → `EXECUTING` | Execution started | All deal parties | `DealExecuting` / Normal |
| `ExecuteTransition` → `COMPLETED` | Deal completed | All deal parties | `DealCompleted` / High |
| `ExecuteTransition` → `CANCELLED` / `EXPIRED` | Deal ended | All deal parties | `DealCancelled` / Normal |
| `ExecuteTransition` → `DISPUTED` | Dispute opened | All deal parties | `DealDisputed` / Critical |

### 8.2 Negotiation / Terms

| Trigger location | Event | Recipients |
|------------------|-------|------------|
| `ProposeTerm::execute` | New term proposed | Other deal parties | `TermProposed` / Normal |
| `AcceptTerm::execute` | Term accepted | Term author + other parties | `TermAccepted` / Normal |
| `RejectTerm::execute` | Term rejected | Term author | `TermRejected` / Normal |
| `CounterTerm::execute` | Term countered | Original proposer | `TermCountered` / Normal |

### 8.3 Milestones

| Trigger location | Event | Recipients |
|------------------|-------|------------|
| `CreateMilestone::execute` | Milestone assigned | Assigned party | `MilestoneAssigned` / Normal |
| `StartMilestone::execute` | Milestone started | Other parties | `MilestoneStarted` / Normal |
| `CompleteMilestone::execute` | Milestone completed | Verifier | `MilestoneCompleted` / Normal |
| `VerifyMilestone::execute` | Milestone verified | All parties + payment release | `MilestoneVerified` / High |
| Deal timeout worker (near due date) | Milestone due soon | Assigned party | `MilestoneDue` / High |

### 8.4 Payments

| Trigger location | Event | Recipients |
|------------------|-------|------------|
| `HoldEscrow::execute` | Escrow funded | All deal parties | `EscrowFunded` / High |
| `ReleaseEscrow::execute` | Escrow released | Receiving parties | `EscrowReleased` / High |
| `ApproveTransaction::execute` | Transaction approved | All involved parties | `TransactionApproved` / Normal |
| Wallet repository on rejection | Transaction rejected | Initiator | `TransactionRejected` / High |
| Worker before deadline | Payment due | Obligated party | `PaymentDue` / High |

### 8.5 Reviews, Disputes, Verifications, Trust

| Trigger location | Event | Recipients |
|------------------|-------|------------|
| `ExecuteTransition` → `COMPLETED` | Review requested | All parties | `ReviewRequested` / Normal |
| `SubmitReview::execute` | New review received | Reviewed party | `ReviewReceived` / Normal |
| `RaiseDispute::execute` | Dispute opened | All parties + admins | `DisputeOpened` / Critical |
| `ResolveDispute::execute` | Dispute resolved | All parties | `DisputeResolved` / High |
| `ApproveVerification::execute` | Verification approved | Party members | `VerificationApproved` / Normal |
| `RejectVerification::execute` | Verification rejected | Party members | `VerificationRejected` / Normal |
| `RecalculateTrustScore::execute` | Trust score changed | Party members | `TrustScoreUpdated` / Low |

### 8.6 Messaging

| Trigger location | Event | Recipients |
|------------------|-------|------------|
| `SendMessage::execute` | Message received | Recipient user/party | `MessageReceived` / Normal |
| `MarkRead::execute` | Read receipt published (existing) | — | No new notification; keep existing `MessageEvent`. |

### 8.7 Admin Broadcast

The existing `AdminBroadcast` message use case can optionally also create a `Notification` row of type `AdminBroadcast` so it appears in the notification center with action links.

---

## 9. Database Schema

New migration (example timestamp): `migrations/20260617000000_create_notifications.sql`

```sql
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    party_id UUID REFERENCES parties(id) ON DELETE CASCADE,
    notification_type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    channels TEXT[] NOT NULL DEFAULT '{}',
    priority TEXT NOT NULL CHECK (priority IN ('LOW','NORMAL','HIGH','CRITICAL')),
    status TEXT NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING','SENT','DELIVERED','FAILED','SUPPRESSED')),
    read_at TIMESTAMPTZ,
    actioned_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    action_url TEXT,
    actions JSONB NOT NULL DEFAULT '[]',
    related_entity_type TEXT,
    related_entity_id UUID,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT notifications_recipient_check CHECK (
        (user_id IS NOT NULL) OR (party_id IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_notifications_user
    ON notifications(user_id, created_at DESC) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_notifications_party
    ON notifications(party_id, created_at DESC) WHERE party_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_notifications_status
    ON notifications(status) WHERE status IN ('PENDING','SENT');
CREATE INDEX IF NOT EXISTS idx_notifications_related
    ON notifications(related_entity_type, related_entity_id);

CREATE TABLE IF NOT EXISTS notification_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    channels JSONB NOT NULL DEFAULT '{"in_app":true,"email":true,"push":false,"sms":false}',
    per_type JSONB NOT NULL DEFAULT '{}',
    quiet_hours JSONB NOT NULL DEFAULT '{"enabled":false,"start":"22:00","end":"07:00","timezone":"UTC","except_critical":true}',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS notification_templates (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    notification_type TEXT NOT NULL,
    channel TEXT NOT NULL,
    locale TEXT NOT NULL DEFAULT 'en',
    subject_template TEXT NOT NULL,
    body_template TEXT NOT NULL,
    variables_schema JSONB NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(notification_type, channel, locale)
);

CREATE INDEX IF NOT EXISTS idx_notification_templates_lookup
    ON notification_templates(notification_type, channel, locale) WHERE is_active = true;

CREATE TABLE IF NOT EXISTS notification_delivery_records (
    id UUID PRIMARY KEY,
    notification_id UUID NOT NULL REFERENCES notifications(id) ON DELETE CASCADE,
    channel TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('PENDING','SENT','DELIVERED','FAILED')),
    attempted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    delivered_at TIMESTAMPTZ,
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    provider_reference TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_delivery_records_notification
    ON notification_delivery_records(notification_id, channel);
```

Seed default templates for `en` in the same migration or a follow-up seed migration.

---

## 10. Authorization Scopes

Add to `role_definitions` seed data (or via migration):

| Scope | Purpose |
|-------|---------|
| `notifications:read` | View own/user/party notifications and preferences. |
| `notifications:write` | Mark read/actioned, delete own notifications, update own preferences. |
| `admin:notifications` | Send admin notifications and manage templates. Fallback `admin:*` always works. |

The `user` role receives `notifications:read` and `notifications:write` by default.

---

## 11. Wire-Up in `main.rs` and `AppState`

### 11.1 New `AppState` fields

```rust
pub struct AppState {
    // ... existing fields ...
    pub list_notifications: application::notifications::ListNotifications,
    pub get_notification: application::notifications::GetNotification,
    pub mark_notification_read: application::notifications::MarkNotificationRead,
    pub mark_all_notifications_read: application::notifications::MarkAllNotificationsRead,
    pub delete_notification: application::notifications::DeleteNotification,
    pub get_unread_notification_count: application::notifications::GetUnreadCount,
    pub get_notification_preferences: application::notifications::GetNotificationPreferences,
    pub update_notification_preferences: application::notifications::UpdateNotificationPreferences,
    pub admin_send_notification: application::notifications::AdminSendNotification,
    pub admin_list_templates: application::notifications::AdminListTemplates,
    pub admin_create_template: application::notifications::AdminCreateTemplate,
    pub admin_update_template: application::notifications::AdminUpdateTemplate,
    pub admin_delete_template: application::notifications::AdminDeleteTemplate,
    pub send_notification: Arc<application::notifications::SendNotification>, // shared with lifecycle use cases
}
```

### 11.2 Wiring in `main.rs`

```rust
let notification_repo: Arc<dyn NotificationRepository> =
    Arc::new(PostgresNotificationRepository::new(pool.clone()));
let notification_pref_repo: Arc<dyn NotificationPreferenceRepository> =
    Arc::new(PostgresNotificationPreferenceRepository::new(pool.clone()));
let notification_template_repo: Arc<dyn NotificationTemplateRepository> =
    Arc::new(PostgresNotificationTemplateRepository::new(pool.clone()));

let push_sender: Arc<dyn PushNotificationSender> = Arc::new(NoOpPushSender);
let sms_sender: Arc<dyn SmsSender> = Arc::new(NoOpSmsSender);

let notification_publisher: Arc<dyn NotificationRealtimePublisher> = Arc::new(
    NotificationWebSocketPublisher::new(websocket_registry.clone()),
);

let send_notification = Arc::new(SendNotification::new(
    notification_repo.clone(),
    notification_pref_repo.clone(),
    notification_template_repo.clone(),
    user_repo.clone(),
    party_repo.clone(),
    deal_repo.clone(),
    email_queue.clone(),
    notification_publisher.clone(),
    push_sender.clone(),
    sms_sender.clone(),
    settings.notifications.default_locale.clone(),
));

// Spawn worker
tokio::spawn(run_notification_worker(
    notification_repo.clone(),
    email_queue.clone(), // worker may also re-enqueue/dispatch directly
    push_sender.clone(),
    sms_sender.clone(),
    settings.notifications.worker_interval_seconds,
    settings.notifications.worker_batch_size,
));
```

Existing lifecycle use cases receive `send_notification.clone()` in their constructors.

---

## 12. Event-Driven Notification Trigger Strategy

Because there is no general event bus today, the pragmatic approach for the monolith is:

1. Introduce a lightweight `DomainEvent` enum in `crates/domain/src/events.rs`:
   ```rust
   pub enum DomainEvent {
       DealCreated { deal_id: Uuid, actor_party_id: Uuid },
       DealStateChanged { deal_id: Uuid, from: DealStatus, to: DealStatus },
       TermProposed { deal_id: Uuid, term_id: Uuid, proposer_party_id: Uuid },
       MilestoneCompleted { deal_id: Uuid, milestone_id: Uuid },
       EscrowReleased { deal_id: Uuid, transaction_id: Uuid },
       DisputeRaised { deal_id: Uuid, dispute_id: Uuid },
       ReviewSubmitted { deal_id: Uuid, review_id: Uuid },
       // ... etc.
   }
   ```
2. Add an outbound port `DomainEventPublisher` in `crates/application/src/ports.rs`:
   ```rust
   #[async_trait]
   pub trait DomainEventPublisher: Send + Sync {
       async fn publish(&self, event: DomainEvent) -> Result<(), ApplicationError>;
   }
   ```
3. Implement it in infrastructure as a synchronous dispatcher that maps each `DomainEvent` to a `SendNotificationCommand` and calls `SendNotification::execute`. This keeps event emission decoupled from notification logic while avoiding an external broker.
4. Existing use cases publish one `DomainEvent` after the database transaction succeeds.
5. Future migration to Kafka/RabbitMQ only changes the `DomainEventPublisher` adapter.

This satisfies the PDF's "Event-Driven Core" principle without adding operational complexity in the monolith phase.

---

## 13. Push Notification Details

### 13.1 Device Tokens

Add a `user_push_tokens` table:

```sql
CREATE TABLE IF NOT EXISTS user_push_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_token TEXT NOT NULL,
    provider TEXT NOT NULL DEFAULT 'FCM', -- 'FCM' | 'APNS'
    device_type TEXT, -- 'ios', 'android', 'web'
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ,
    UNIQUE(user_id, device_token)
);
```

Add repository port and a `RegisterPushToken` use case + endpoint:

```
POST /api/v1/notifications/push-tokens
DELETE /api/v1/notifications/push-tokens/{token}
```

### 13.2 Provider Adapter

`NoOpPushSender` for tests/local. Production adapter:
- FCM: HTTP v1 API via `reqwest` with service-account JWT.
- APNs: HTTP/2 provider API via `reqwest` or `apns2` crate.

### 13.3 In-App Real-Time Delivery

The existing WebSocket registry is user-centric. A `NotificationWebSocketPublisher` maps:

```rust
NotificationEvent::NotificationNew { user_id, party_id, .. } => {
    if let Some(uid) = user_id { registry.notify_user(uid, event); }
    if let Some(pid) = party_id { registry.notify_party_members(pid, event); }
}
```

The current `SessionRegistry` only stores by `user_id`. For party-level in-app delivery, look up party members via `PartyRepository::list_members` and notify each connected user.

---

## 14. Admin Notification Console

### 14.1 Send Custom Notification

```
POST /api/v1/admin/notifications/send
```

Request body:

```json
{
  "target": {
    "type": "ALL_USERS" | "ALL_PARTIES" | "USER" | "PARTY" | "PARTY_MEMBERS" | "DEAL_PARTICIPANTS",
    "user_id": "...",
    "party_id": "...",
    "deal_id": "..."
  },
  "notification": {
    "type": "AdminBroadcast",
    "priority": "HIGH",
    "title": "Scheduled Maintenance",
    "body": "Platform maintenance on Sunday 02:00–04:00 UTC.",
    "action_url": "/maintenance",
    "channels": ["IN_APP", "EMAIL"],
    "expires_at": "2026-06-20T04:00:00Z"
  }
}
```

Response: `202 Accepted` with `sent_count` and `notification_ids` (or job id for large batches).

### 14.2 Template Management

```
GET    /api/v1/admin/notification-templates
POST   /api/v1/admin/notification-templates
GET    /api/v1/admin/notification-templates/{id}
PUT    /api/v1/admin/notification-templates/{id}
DELETE /api/v1/admin/notification-templates/{id}
```

Template body uses `handlebars` or `tinytemplate` variable substitution. Variables are validated against `variables_schema`.

---

## 15. Testing Strategy (≥ 85 % Coverage)

### 15.1 Test Layers

| Layer | Scope | Tooling |
|-------|-------|---------|
| Domain unit tests | `Notification`, `NotificationPreference`, routing/quiet-hours logic | `cargo test` inline in source files |
| Application unit tests | Use cases with fake repositories and fakes for email/push/sms/realtime | `application::test_helpers` extensions |
| Infrastructure integration tests | Postgres repositories + worker with testcontainers/local PG | `crates/infrastructure/tests/postgres_notifications.rs` |
| API integration tests | Full HTTP handlers with in-memory state | `crates/api/tests/notifications.rs` |

### 15.2 Fake Test Doubles to Add

In `crates/application/src/test_helpers.rs`:

```rust
#[derive(Default)]
pub struct FakeNotificationRepository { notifications: Mutex<HashMap<Uuid, Notification>> }
#[derive(Default)]
pub struct FakeNotificationPreferenceRepository { prefs: Mutex<HashMap<Uuid, NotificationPreference>> }
#[derive(Default)]
pub struct FakeNotificationTemplateRepository { templates: Mutex<HashMap<Uuid, NotificationTemplate>> }
#[derive(Default)]
pub struct FakePushSender { calls: Mutex<Vec<PushCall>> }
#[derive(Default)]
pub struct FakeSmsSender { calls: Mutex<Vec<SmsCall>> }
#[derive(Default)]
pub struct RecordingNotificationPublisher { events: Mutex<Vec<NotificationEvent>> }
```

### 15.3 Key Test Scenarios

1. **Routing & preferences**
   - Critical notification bypasses quiet hours.
   - User with email disabled receives only in-app.
   - Disabled notification type is suppressed.
2. **Quiet hours**
   - Normal notification scheduled within quiet hours is suppressed for push/email.
   - Critical notification still delivered if `except_critical = true`.
3. **Template rendering**
   - Template variables substituted correctly.
   - Missing template falls back to default locale.
   - Unknown template returns validation error.
4. **Recipient resolution**
   - Deal participants resolved correctly.
   - Party members expanded.
   - Admin broadcast creates one notification per user/party.
5. **Lifecycle integration**
   - `SubmitDeal` emits notifications to other parties.
   - `VerifyMilestone` emits `MilestoneVerified` to all parties.
   - `RaiseDispute` emits critical notification.
6. **In-app real-time**
   - `NotificationEvent` published on create and read.
   - Unread count event emitted after mark-read.
7. **Worker delivery**
   - Pending notification is picked up and dispatched.
   - Failed channel retries then marks failed.
8. **API authorization**
   - User cannot read another user's notifications.
   - Admin can send platform notifications.
   - Missing scope returns 403.
9. **Edge cases**
   - Expired notification is not delivered.
   - Duplicate notification idempotency via `idempotency_key` (optional).

### 15.4 Coverage Measurement

Run:

```bash
cargo llvm-cov --workspace --html
```

Target:
- Domain notification modules: ≥ 90 %
- Application notification use cases: ≥ 90 %
- Postgres notification repositories: ≥ 80 %
- API notification handlers: ≥ 85 %
- Overall notification feature: ≥ 85 %

### 15.5 CI

Add notification migration and tests to the existing `.github/workflows/ci.yml` — no new workflow needed.

---

## 16. Migration & Rollout Plan

1. **Migration 1:** Create tables + indexes + seed default templates.
2. **Migration 2:** Add notification scopes to `role_definitions` seed.
3. **Code PR 1:** Domain entities + repository ports + Postgres implementations + application use cases (no lifecycle integration).
4. **Code PR 2:** API routes/handlers + WebSocket publisher extension.
5. **Code PR 3:** Lifecycle event integration (deal, term, milestone, payment, dispute, review, verification).
6. **Code PR 4:** Background worker + push/SMS stubs + device-token registration.
7. **Code PR 5:** Admin send/template management.
8. **QA:** Run full test suite, measure coverage, add tests until ≥ 85 %.

All migrations must be **idempotent** (`IF NOT EXISTS`) and **backwards-compatible** per `AGENTS.md`.

---

## 17. API Summary

### User / Party Facing

| Method | Endpoint | Auth scope | Description |
|--------|----------|------------|-------------|
| GET | `/api/v1/notifications` | `notifications:read` | List with filters (type, read, priority). |
| GET | `/api/v1/notifications/{id}` | `notifications:read` | Get single notification. |
| PATCH | `/api/v1/notifications/{id}` | `notifications:write` | Mark read or actioned. |
| DELETE | `/api/v1/notifications/{id}` | `notifications:write` | Delete own notification. |
| POST | `/api/v1/notifications/actions/mark-all-read` | `notifications:write` | Bulk mark read. |
| GET | `/api/v1/notifications/unread-count` | `notifications:read` | Unread badge count. |
| GET | `/api/v1/notifications/preferences` | `notifications:read` | Get preferences. |
| PUT | `/api/v1/notifications/preferences` | `notifications:write` | Update preferences. |
| POST | `/api/v1/notifications/push-tokens` | `notifications:write` | Register device token. |
| DELETE | `/api/v1/notifications/push-tokens/{token}` | `notifications:write` | Unregister token. |

### Admin / Scoped Management

| Method | Endpoint | Auth scope | Description |
|--------|----------|------------|-------------|
| POST | `/api/v1/admin/notifications/send` | `admin:notifications` or `admin:*` | Send custom/platform notification. |
| GET | `/api/v1/admin/notification-templates` | `admin:notifications` | List templates. |
| POST | `/api/v1/admin/notification-templates` | `admin:notifications` | Create template. |
| GET | `/api/v1/admin/notification-templates/{id}` | `admin:notifications` | Get template. |
| PUT | `/api/v1/admin/notification-templates/{id}` | `admin:notifications` | Update template. |
| DELETE | `/api/v1/admin/notification-templates/{id}` | `admin:notifications` | Deactivate/delete template. |

---

## 18. Security & Privacy Considerations

- **PII in metadata:** Avoid storing free-form PII in `metadata` JSONB; prefer entity IDs and fetch details at render time.
- **Authorization:** Every repository read filters by `user_id` or `party_id`; admins do not bypass notification ownership unless explicitly using admin endpoints.
- **Quiet hours:** Critical security alerts (login from new device, password reset, dispute) bypass quiet hours.
- **Email unsubscribe:** Marketing/admin broadcast emails include an unsubscribe link to the preferences page.
- **Push token hygiene:** Tokens are per-user; stale tokens removed after provider returns `NotRegistered`.
- **Rate limiting:** Admin send endpoint should be rate-limited separately to prevent abuse.

---

## 19. Metrics & Observability

Emit structured logs/traces:

```rust
tracing::info!(
    notification_id = %notification.id,
    user_id = ?notification.user_id,
    party_id = ?notification.party_id,
    notification_type = %notification.notification_type.as_str(),
    channels = ?notification.channels,
    "notification created"
);
```

Counters (future Prometheus):
- `notifications_created_total` by type, priority.
- `notifications_delivered_total` by channel.
- `notifications_failed_total` by channel, error.
- `notification_worker_poll_duration_seconds`.

---

## 20. Files to Create / Modify

### New files
- `crates/domain/src/entities/notification.rs`
- `crates/domain/src/entities/notification_preference.rs`
- `crates/domain/src/entities/notification_template.rs`
- `crates/domain/src/repositories/notification_repository.rs`
- `crates/application/src/notifications/mod.rs`
- `crates/application/src/notifications/dto.rs`
- `crates/application/src/notifications/errors.rs` (if needed)
- `crates/application/src/notifications/create_notification.rs`
- `crates/application/src/notifications/send_notification.rs`
- `crates/application/src/notifications/list_notifications.rs`
- `crates/application/src/notifications/get_notification.rs`
- `crates/application/src/notifications/mark_read.rs`
- `crates/application/src/notifications/mark_all_read.rs`
- `crates/application/src/notifications/delete_notification.rs`
- `crates/application/src/notifications/get_unread_count.rs`
- `crates/application/src/notifications/get_preferences.rs`
- `crates/application/src/notifications/update_preferences.rs`
- `crates/application/src/notifications/admin_send.rs`
- `crates/application/src/notifications/admin_templates.rs`
- `crates/application/src/notifications/render.rs`
- `crates/application/src/notifications/route.rs`
- `crates/application/src/notifications/tests.rs`
- `crates/infrastructure/src/repositories/postgres_notification_repository.rs`
- `crates/infrastructure/src/repositories/postgres_notification_preference_repository.rs`
- `crates/infrastructure/src/repositories/postgres_notification_template_repository.rs`
- `crates/infrastructure/src/notifications/mod.rs`
- `crates/infrastructure/src/notifications/noop_push_sender.rs`
- `crates/infrastructure/src/notifications/noop_sms_sender.rs`
- `crates/infrastructure/src/notifications/notification_worker.rs`
- `crates/infrastructure/src/realtime/notification_publisher.rs`
- `crates/api/src/handlers/notifications/mod.rs`
- `crates/api/src/handlers/notifications/list.rs`
- `crates/api/src/handlers/notifications/get.rs`
- `crates/api/src/handlers/notifications/update.rs`
- `crates/api/src/handlers/notifications/delete.rs`
- `crates/api/src/handlers/notifications/mark_all_read.rs`
- `crates/api/src/handlers/notifications/unread_count.rs`
- `crates/api/src/handlers/notifications/preferences.rs`
- `crates/api/src/handlers/notifications/admin_send.rs`
- `crates/api/src/handlers/notifications/admin_templates.rs`
- `crates/api/src/routes/notifications.rs`
- `migrations/20260617000000_create_notifications.sql`
- `crates/infrastructure/tests/postgres_notification_repository.rs`
- `crates/api/tests/notifications.rs`

### Modified files
- `crates/domain/src/entities/mod.rs` — register new entities.
- `crates/domain/src/repositories/mod.rs` — register new ports.
- `crates/application/src/lib.rs` — expose `notifications` module.
- `crates/application/src/ports.rs` — add `NotificationEvent`, `NotificationRealtimePublisher`, `PushNotificationSender`, `SmsSender`, `DomainEventPublisher`.
- `crates/application/src/test_helpers.rs` — add fake repositories and sender fakes.
- `crates/application/src/deals/*.rs`, `milestones/*.rs`, `payments/*.rs`, `disputes/*.rs`, `reviews/*.rs`, `verifications/*.rs`, `trust_scores/*.rs`, `messages/*.rs` — inject `DomainEventPublisher` and emit events.
- `crates/infrastructure/src/lib.rs` — expose new modules.
- `crates/infrastructure/src/config.rs` — add `NotificationSettings`.
- `crates/infrastructure/src/realtime/mod.rs` — expose notification publisher.
- `crates/infrastructure/src/workers/mod.rs` — expose notification worker.
- `crates/api/src/lib.rs` — add new fields to `AppState`.
- `crates/api/src/main.rs` — wire repositories, publishers, senders, worker.
- `crates/api/src/routes/mod.rs` — mount notification routes.
- `crates/api/src/websocket/message_socket.rs` — add `NotificationNew` / `NotificationRead` / `UnreadCountChanged` WS events.
- `crates/api/tests/common/mod.rs` — wire notification fakes in test `AppState`.
- `.env.example` — add notification settings.
- `.github/workflows/ci.yml` — ensure new migrations/tests run.

---

## 21. Alignment with `3partydeal.pdf`

This design directly implements the PDF's:

- **§3.23 Notification** entity (channels, priority, read/actioned, expiry, related entity).
- **§5.2.11 Notification Service** responsibilities (multi-channel, preferences, templating, delivery tracking, rate limiting).
- **§5.5 Notification Channels** strategy and routing table.
- **§6.11 Notification APIs** endpoints for list, read, preferences, admin send, templates.
- **§6.12 Event Schema** by mapping deal/negotiation/milestone/payment/trust events to notification triggers.
- **§7.7 Notification Design** wireframes (notification center, preferences UI, urgency indicators, SMS rules).
- **Appendix C Notification Matrix** for deal/milestone/dispute/payment lifecycle notifications.

It intentionally defers Kafka/RabbitMQ, GraphQL, gRPC, and external webhook delivery to later phases while keeping the internal event envelope ready for those transports.

---

## 22. Conclusion

The notification subsystem should be implemented as a first-class domain inside the existing hexagonal monolith:

1. **Domain first** — `Notification`, `NotificationPreference`, `NotificationTemplate` with strict validation.
2. **Ports second** — repositories and delivery adapters abstract PostgreSQL, email, push, SMS, and real-time channels.
3. **Use cases third** — small, testable services for send/list/read/preferences/admin/templates.
4. **Triggers fourth** — existing deal/milestone/payment/dispute/review/verification use cases publish domain events that the notification dispatcher consumes.
5. **API fifth** — REST endpoints and WebSocket events for the notification center.
6. **Tests throughout** — domain unit tests, application tests with fakes, Postgres integration tests, and API integration tests to exceed 85 % coverage.

This approach gives Hayaland a production-ready notification service with minimal architectural disruption and a clear upgrade path to a distributed event bus.
