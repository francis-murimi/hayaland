use crate::errors::ApplicationError;
use crate::matching::dto::{ListMatchesQuery, MatchSuggestionResult};
use crate::matching::generate_matches::map_to_result;
use domain::repositories::{MatchFilters, MatchRepository, PartyRepository};
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

/// List match suggestions for a party or across the platform (admin).
#[derive(Clone)]
pub struct ListMatches {
    match_repo: Arc<dyn MatchRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl ListMatches {
    pub fn new(match_repo: Arc<dyn MatchRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            match_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, query))]
    pub async fn execute(
        &self,
        actor_user_id: Uuid,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
        query: ListMatchesQuery,
    ) -> Result<Vec<MatchSuggestionResult>, ApplicationError> {
        let filters = MatchFilters {
            status: query.status,
            min_score: query.min_score,
            max_score: query.max_score,
            limit: query.limit,
            offset: query.offset,
            ..MatchFilters::default()
        };

        let suggestions = if is_admin && actor_party_id.is_none() {
            self.match_repo.list_all(&filters).await?
        } else {
            let party_id = actor_party_id.ok_or(ApplicationError::Forbidden)?;
            self.verify_membership(actor_user_id, party_id).await?;
            self.match_repo
                .list_for_party(party_id, query.role, &filters)
                .await?
        };

        Ok(suggestions.into_iter().map(map_to_result).collect())
    }

    async fn verify_membership(
        &self,
        user_id: Uuid,
        party_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let is_member = self
            .party_repo
            .is_user_member_of_party(user_id, party_id)
            .await?;
        if is_member {
            Ok(())
        } else {
            Err(ApplicationError::Forbidden)
        }
    }
}
