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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::errors::DomainError;
    use domain::repositories::{AuditLogFilters, AuditLogListResult};
    use std::sync::Mutex;

    struct FakeAuditRepo {
        created: Mutex<Vec<AdminAction>>,
    }

    impl FakeAuditRepo {
        fn new() -> Self {
            Self {
                created: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl AuditLogRepository for FakeAuditRepo {
        async fn create(&self, action: &AdminAction) -> Result<(), DomainError> {
            self.created.lock().unwrap().push(action.clone());
            Ok(())
        }

        async fn list(&self, filters: &AuditLogFilters) -> Result<AuditLogListResult, DomainError> {
            let created = self.created.lock().unwrap();
            let items: Vec<_> = created
                .iter()
                .filter(|a| {
                    filters
                        .admin_user_id
                        .map(|id| a.admin_user_id == id)
                        .unwrap_or(true)
                })
                .cloned()
                .collect();
            Ok(AuditLogListResult {
                total: items.len() as i64,
                items,
            })
        }

        async fn list_for_target(
            &self,
            _target_type: AdminActionTargetType,
            _target_id: Uuid,
            _limit: i64,
            _offset: i64,
        ) -> Result<AuditLogListResult, DomainError> {
            Ok(AuditLogListResult {
                items: vec![],
                total: 0,
            })
        }
    }

    fn command() -> RecordAdminActionCommand {
        RecordAdminActionCommand {
            admin_user_id: Uuid::now_v7(),
            action_type: "PARTY_UPDATED".to_string(),
            target_type: "PARTY".to_string(),
            target_id: Uuid::now_v7(),
            before_snapshot: Some(serde_json::json!({"name": "old"})),
            after_snapshot: Some(serde_json::json!({"name": "new"})),
            reason: Some("cleanup".to_string()),
            ip_address: Some("127.0.0.1".to_string()),
        }
    }

    #[tokio::test]
    async fn record_admin_action_persists_and_maps() {
        let repo = Arc::new(FakeAuditRepo::new());
        let uc = RecordAdminAction::new(repo.clone());
        let cmd = command();
        let admin_id = cmd.admin_user_id;
        let result = uc.execute(cmd).await.unwrap();
        assert_eq!(result.admin_user_id, admin_id);
        assert_eq!(result.action_type, "PARTY_UPDATED");
        assert_eq!(result.target_type, "PARTY");
        assert_eq!(result.reason.as_deref(), Some("cleanup"));
        assert_eq!(result.ip_address.as_deref(), Some("127.0.0.1"));
        assert!(result.before_snapshot.is_some());
        assert!(result.after_snapshot.is_some());
        assert_eq!(repo.created.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn record_admin_action_rejects_bad_action_type() {
        let uc = RecordAdminAction::new(Arc::new(FakeAuditRepo::new()));
        let mut cmd = command();
        cmd.action_type = "NOPE".to_string();
        let err = uc.execute(cmd).await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn record_admin_action_rejects_bad_target_type() {
        let uc = RecordAdminAction::new(Arc::new(FakeAuditRepo::new()));
        let mut cmd = command();
        cmd.target_type = "NOPE".to_string();
        let err = uc.execute(cmd).await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn list_audit_log_applies_filters_and_clamps() {
        let repo = Arc::new(FakeAuditRepo::new());
        let uc = RecordAdminAction::new(repo.clone());
        let cmd = command();
        let admin_id = cmd.admin_user_id;
        uc.execute(cmd).await.unwrap();

        let list = ListAuditLog::new(repo);
        let result = list
            .execute(AuditLogFiltersDto {
                admin_user_id: Some(admin_id),
                action_type: Some("PARTY_UPDATED".to_string()),
                target_type: Some("PARTY".to_string()),
                target_id: None,
                from: None,
                to: None,
                limit: Some(500),
                offset: Some(-1),
            })
            .await
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].action_type, "PARTY_UPDATED");
    }

    #[tokio::test]
    async fn list_audit_log_rejects_invalid_filter_values() {
        let list = ListAuditLog::new(Arc::new(FakeAuditRepo::new()));
        let err = list
            .execute(AuditLogFiltersDto {
                action_type: Some("NOPE".to_string()),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));

        let err = list
            .execute(AuditLogFiltersDto {
                target_type: Some("NOPE".to_string()),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }
}
