pub mod dto;

use crate::audit_log::dto::{
    AdminActionResult, AuditLogFiltersDto, AuditLogListDto, RecordAdminActionCommand,
};
use crate::errors::ApplicationError;
use domain::entities::{AdminAction, AdminActionTargetType, AdminActionType};
use domain::repositories::{AuditLogFilters, AuditLogRepository};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
pub struct RecordAdminAction {
    repo: Arc<dyn AuditLogRepository>,
}

impl RecordAdminAction {
    pub fn new(repo: Arc<dyn AuditLogRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        cmd: RecordAdminActionCommand,
    ) -> Result<AdminActionResult, ApplicationError> {
        let action_type =
            AdminActionType::try_from(cmd.action_type.as_str()).map_err(ApplicationError::from)?;
        let target_type = AdminActionTargetType::try_from(cmd.target_type.as_str())
            .map_err(ApplicationError::from)?;

        let action = AdminAction::new(
            Uuid::now_v7(),
            cmd.admin_user_id,
            action_type,
            target_type,
            cmd.target_id,
            cmd.before_snapshot,
            cmd.after_snapshot,
            cmd.reason,
            cmd.ip_address,
        );

        self.repo.create(&action).await?;
        info!(
            admin_user_id = %action.admin_user_id,
            action_type = %action.action_type.as_str(),
            target_id = %action.target_id,
            "recorded admin action"
        );
        Ok(map_admin_action(action))
    }
}

#[derive(Clone)]
pub struct ListAuditLog {
    repo: Arc<dyn AuditLogRepository>,
}

impl ListAuditLog {
    pub fn new(repo: Arc<dyn AuditLogRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        filters: AuditLogFiltersDto,
    ) -> Result<AuditLogListDto, ApplicationError> {
        let action_type = filters
            .action_type
            .as_deref()
            .map(AdminActionType::try_from)
            .transpose()
            .map_err(ApplicationError::from)?;
        let target_type = filters
            .target_type
            .as_deref()
            .map(AdminActionTargetType::try_from)
            .transpose()
            .map_err(ApplicationError::from)?;

        let filters = AuditLogFilters {
            admin_user_id: filters.admin_user_id,
            action_type,
            target_type,
            target_id: filters.target_id,
            from: filters.from,
            to: filters.to,
            limit: filters.limit.unwrap_or(20).clamp(1, 100),
            offset: filters.offset.unwrap_or(0).max(0),
        };

        let result = self.repo.list(&filters).await?;
        Ok(AuditLogListDto {
            items: result.items.into_iter().map(map_admin_action).collect(),
            total: result.total,
        })
    }
}

fn map_admin_action(action: AdminAction) -> AdminActionResult {
    AdminActionResult {
        id: action.id,
        admin_user_id: action.admin_user_id,
        action_type: action.action_type.as_str().to_string(),
        target_type: action.target_type.as_str().to_string(),
        target_id: action.target_id,
        before_snapshot: action.before_snapshot,
        after_snapshot: action.after_snapshot,
        reason: action.reason,
        ip_address: action.ip_address,
        created_at: action.created_at,
    }
}
