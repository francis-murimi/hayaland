use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct AdminActionResult {
    pub id: Uuid,
    pub admin_user_id: Uuid,
    pub action_type: String,
    pub target_type: String,
    pub target_id: Uuid,
    pub before_snapshot: Option<serde_json::Value>,
    pub after_snapshot: Option<serde_json::Value>,
    pub reason: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecordAdminActionCommand {
    #[serde(default)]
    pub admin_user_id: Uuid,
    pub action_type: String,
    pub target_type: String,
    pub target_id: Uuid,
    pub before_snapshot: Option<serde_json::Value>,
    pub after_snapshot: Option<serde_json::Value>,
    pub reason: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuditLogFiltersDto {
    pub admin_user_id: Option<Uuid>,
    pub action_type: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub from: Option<OffsetDateTime>,
    pub to: Option<OffsetDateTime>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditLogListDto {
    pub items: Vec<AdminActionResult>,
    pub total: i64,
}
