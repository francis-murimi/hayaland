use domain::entities::{
    ActionType, Notification, NotificationAction, NotificationChannel, NotificationPriority,
    NotificationType,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Recipient target for sending a notification.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipientSelector {
    User { user_id: Uuid },
    Party { party_id: Uuid },
    PartyMembers { party_id: Uuid },
    DealParticipants { deal_id: Uuid },
    AllUsers,
    AllParties,
}

/// Command to create/send a notification.
#[derive(Debug, Clone, Deserialize)]
pub struct SendNotificationCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub recipient: RecipientSelector,
    pub notification_type: NotificationType,
    pub priority: NotificationPriority,
    pub title: Option<String>,
    pub body: Option<String>,
    pub action_url: Option<String>,
    pub actions: Vec<NotificationActionDto>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<Uuid>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_locale() -> String {
    "en".to_string()
}

/// Admin-specific request body for sending notifications.
#[derive(Debug, Clone, Deserialize)]
pub struct AdminSendNotificationRequest {
    pub target: RecipientSelector,
    pub notification_type: NotificationType,
    pub priority: NotificationPriority,
    pub title: Option<String>,
    pub body: Option<String>,
    pub action_url: Option<String>,
    #[serde(default)]
    pub actions: Vec<NotificationActionDto>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<Uuid>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub expires_at: Option<OffsetDateTime>,
    #[serde(default = "default_locale")]
    pub locale: String,
}

/// Serialisable notification action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotificationActionDto {
    pub label: String,
    pub action_type: ActionType,
    pub url: Option<String>,
    pub method: Option<String>,
}

impl From<&NotificationAction> for NotificationActionDto {
    fn from(a: &NotificationAction) -> Self {
        Self {
            label: a.label.clone(),
            action_type: a.action_type,
            url: a.url.clone(),
            method: a.method.clone(),
        }
    }
}

impl From<NotificationActionDto> for NotificationAction {
    fn from(dto: NotificationActionDto) -> Self {
        Self {
            label: dto.label,
            action_type: dto.action_type,
            url: dto.url,
            method: dto.method,
        }
    }
}

/// Public result view of a notification.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationResult {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub party_id: Option<Uuid>,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub channels: Vec<String>,
    pub priority: String,
    pub status: String,
    pub is_read: bool,
    pub is_actioned: bool,
    pub expires_at: Option<OffsetDateTime>,
    pub action_url: Option<String>,
    pub actions: Vec<NotificationActionDto>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub created_at: OffsetDateTime,
}

impl From<&Notification> for NotificationResult {
    fn from(n: &Notification) -> Self {
        Self {
            id: n.id,
            user_id: n.user_id,
            party_id: n.party_id,
            notification_type: n.notification_type.as_str().to_string(),
            title: n.title.clone(),
            body: n.body.clone(),
            channels: n.channels.iter().map(|c| c.as_str().to_string()).collect(),
            priority: n.priority.as_str().to_string(),
            status: n.status.as_str().to_string(),
            is_read: n.read_at.is_some(),
            is_actioned: n.actioned_at.is_some(),
            expires_at: n.expires_at,
            action_url: n.action_url.clone(),
            actions: n.actions.iter().map(NotificationActionDto::from).collect(),
            related_entity_type: n.related_entity_type.clone(),
            related_entity_id: n.related_entity_id,
            metadata: n.metadata.clone(),
            created_at: n.created_at,
        }
    }
}

/// Result of a list query.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationListResultDto {
    pub data: Vec<NotificationResult>,
    pub unread_count: i64,
    pub total: i64,
}

/// Filters for listing notifications.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct NotificationListQuery {
    pub notification_type: Option<NotificationType>,
    pub is_read: Option<bool>,
    pub is_actioned: Option<bool>,
    pub priority: Option<NotificationPriority>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Body for marking a notification as read/actioned.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateNotificationBody {
    pub is_read: Option<bool>,
    pub is_actioned: Option<bool>,
}

/// Body for bulk marking notifications as read.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MarkAllReadBody {
    pub notification_type: Option<NotificationType>,
    pub before_date: Option<OffsetDateTime>,
}

/// Unread count response.
#[derive(Debug, Clone, Serialize)]
pub struct UnreadCountResult {
    pub count: i64,
}

/// Channel preferences DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPreferencesDto {
    pub in_app: bool,
    pub email: bool,
    pub push: bool,
    pub sms: bool,
}

/// Per-type preference DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypePreferenceDto {
    pub enabled: bool,
    pub channels: Vec<NotificationChannel>,
}

/// Quiet hours DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHoursDto {
    pub enabled: bool,
    pub start: String,
    pub end: String,
    pub timezone: String,
    pub except_critical: bool,
}

/// Full notification preferences DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferencesDto {
    pub user_id: Uuid,
    pub channels: ChannelPreferencesDto,
    pub per_type: serde_json::Value,
    pub quiet_hours: QuietHoursDto,
}

/// Template creation/update request.
#[derive(Debug, Clone, Deserialize)]
pub struct NotificationTemplateRequest {
    pub name: String,
    pub notification_type: NotificationType,
    pub channel: NotificationChannel,
    pub locale: String,
    pub subject_template: String,
    pub body_template: String,
    #[serde(default)]
    pub variables_schema: serde_json::Value,
}

/// Template result DTO.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationTemplateResult {
    pub id: Uuid,
    pub name: String,
    pub notification_type: String,
    pub channel: String,
    pub locale: String,
    pub subject_template: String,
    pub body_template: String,
    pub variables_schema: serde_json::Value,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<&domain::entities::NotificationTemplate> for NotificationTemplateResult {
    fn from(t: &domain::entities::NotificationTemplate) -> Self {
        Self {
            id: t.id,
            name: t.name.clone(),
            notification_type: t.notification_type.as_str().to_string(),
            channel: t.channel.as_str().to_string(),
            locale: t.locale.clone(),
            subject_template: t.subject_template.clone(),
            body_template: t.body_template.clone(),
            variables_schema: t.variables_schema.clone(),
            is_active: t.is_active,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

/// Admin send response.
#[derive(Debug, Clone, Serialize)]
pub struct AdminSendNotificationResult {
    pub sent_count: usize,
    pub notification_ids: Vec<Uuid>,
}
