use crate::errors::ApplicationError;
use crate::matching::dto::{
    AdminDeleteMatchesCommand, AdminUpdateMatchCommand, MatchStatusCountsResult,
};
use crate::matching::generate_matches::map_to_result;
use domain::entities::MatchSuggestionAuditLogEntry;
use domain::repositories::{MatchFilters, MatchRepository, MatchSuggestionAuditLogRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Administrative controls over match suggestions.
#[derive(Clone)]
pub struct AdminMatchControls {
    match_repo: Arc<dyn MatchRepository>,
    audit_repo: Arc<dyn MatchSuggestionAuditLogRepository>,
}

impl AdminMatchControls {
    pub fn new(
        match_repo: Arc<dyn MatchRepository>,
        audit_repo: Arc<dyn MatchSuggestionAuditLogRepository>,
    ) -> Self {
        Self {
            match_repo,
            audit_repo,
        }
    }

    #[instrument(skip(self))]
    pub async fn list_all(
        &self,
        filters: &MatchFilters,
    ) -> Result<Vec<crate::matching::dto::MatchSuggestionResult>, ApplicationError> {
        let suggestions = self.match_repo.list_all(filters).await?;
        Ok(suggestions.into_iter().map(map_to_result).collect())
    }

    #[instrument(skip(self, cmd))]
    pub async fn update_status(
        &self,
        cmd: AdminUpdateMatchCommand,
    ) -> Result<(), ApplicationError> {
        let before = self.match_repo.find_by_id(cmd.match_suggestion_id).await?;
        if before.is_none() {
            return Err(ApplicationError::NotFound);
        }

        self.match_repo
            .update_status(cmd.match_suggestion_id, cmd.new_status, cmd.reason.clone())
            .await?;

        let after = self.match_repo.find_by_id(cmd.match_suggestion_id).await?;
        let before_snapshot = serde_json::to_value(&before)
            .map_err(|e| ApplicationError::Infrastructure(format!("JSON error: {e}")))?;
        let after_snapshot = serde_json::to_value(&after)
            .map_err(|e| ApplicationError::Infrastructure(format!("JSON error: {e}")))?;

        self.audit_repo
            .create(&MatchSuggestionAuditLogEntry::new(
                Uuid::now_v7(),
                Some(cmd.admin_user_id),
                "UPDATE_STATUS",
                Some(cmd.match_suggestion_id),
                None,
                Some(before_snapshot),
                Some(after_snapshot),
                cmd.reason,
            ))
            .await?;

        info!(
            match_id = %cmd.match_suggestion_id,
            admin = %cmd.admin_user_id,
            ?cmd.new_status,
            "admin updated match suggestion status"
        );
        Ok(())
    }

    #[instrument(skip(self, cmd))]
    pub async fn delete_for_party(
        &self,
        cmd: AdminDeleteMatchesCommand,
    ) -> Result<u64, ApplicationError> {
        let deleted = self
            .match_repo
            .delete_by_party(cmd.party_id, cmd.status)
            .await?;

        let after_snapshot = serde_json::json!({
            "deleted": deleted,
            "status_filter": cmd.status.map(|s| s.as_str().to_string()),
        });
        self.audit_repo
            .create(&MatchSuggestionAuditLogEntry::new(
                Uuid::now_v7(),
                Some(cmd.admin_user_id),
                "DELETE_FOR_PARTY",
                None,
                Some(cmd.party_id),
                None,
                Some(after_snapshot),
                cmd.reason,
            ))
            .await?;

        info!(
            party_id = %cmd.party_id,
            admin = %cmd.admin_user_id,
            deleted,
            "admin deleted match suggestions for party"
        );
        Ok(deleted)
    }

    #[instrument(skip(self))]
    pub async fn delete_all(&self, admin_user_id: Uuid) -> Result<u64, ApplicationError> {
        let deleted = self.match_repo.delete_all().await?;

        let after_snapshot = serde_json::json!({ "deleted": deleted });
        self.audit_repo
            .create(&MatchSuggestionAuditLogEntry::new(
                Uuid::now_v7(),
                Some(admin_user_id),
                "DELETE_ALL",
                None,
                None,
                None,
                Some(after_snapshot),
                None,
            ))
            .await?;

        info!(
            admin = %admin_user_id,
            deleted,
            "admin deleted all match suggestions"
        );
        Ok(deleted)
    }

    #[instrument(skip(self))]
    pub async fn count_for_party(
        &self,
        party_id: Uuid,
    ) -> Result<MatchStatusCountsResult, ApplicationError> {
        let counts = self.match_repo.count_by_status(party_id).await?;
        Ok(counts.into())
    }

    #[instrument(skip(self))]
    pub async fn count_platform(&self) -> Result<MatchStatusCountsResult, ApplicationError> {
        let counts = self.match_repo.count_all_by_status().await?;
        Ok(counts.into())
    }
}
