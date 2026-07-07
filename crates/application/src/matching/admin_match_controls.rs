use crate::errors::ApplicationError;
use crate::matching::dto::{
    AdminDeleteMatchesCommand, AdminUpdateMatchCommand, MatchStatusCountsResult,
};
use crate::matching::generate_matches::map_to_result;
use domain::repositories::{MatchFilters, MatchRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Administrative controls over match suggestions.
#[derive(Clone)]
pub struct AdminMatchControls {
    match_repo: Arc<dyn MatchRepository>,
}

impl AdminMatchControls {
    pub fn new(match_repo: Arc<dyn MatchRepository>) -> Self {
        Self { match_repo }
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
        let exists = self.match_repo.find_by_id(cmd.match_suggestion_id).await?;
        if exists.is_none() {
            return Err(ApplicationError::NotFound);
        }

        self.match_repo
            .update_status(cmd.match_suggestion_id, cmd.new_status, cmd.reason.clone())
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
