use crate::entities::{AdminAction, AdminActionTargetType, AdminActionType};
use crate::errors::DomainError;
use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct AuditLogFilters {
    pub admin_user_id: Option<Uuid>,
    pub action_type: Option<AdminActionType>,
    pub target_type: Option<AdminActionTargetType>,
    pub target_id: Option<Uuid>,
    pub from: Option<OffsetDateTime>,
    pub to: Option<OffsetDateTime>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct AuditLogListResult {
    pub items: Vec<AdminAction>,
    pub total: i64,
}

/// Outbound port for persisting and querying admin audit log entries.
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    /// Record a single admin action.
    async fn create(&self, action: &AdminAction) -> Result<(), DomainError>;

    /// List admin actions with optional filters.
    async fn list(&self, filters: &AuditLogFilters) -> Result<AuditLogListResult, DomainError>;

    /// List actions for a specific target.
    async fn list_for_target(
        &self,
        target_type: AdminActionTargetType,
        target_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<AuditLogListResult, DomainError>;
}
