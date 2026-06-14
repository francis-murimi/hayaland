use crate::errors::ApplicationError;
use crate::parties::create_party::map_party_to_result;
use crate::parties::dto::PartyResult;
use domain::repositories::PartyRepository;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

/// Query for retrieving a single party.
#[derive(Debug, Clone)]
pub struct GetPartyQuery {
    pub actor_user_id: Uuid,
    pub is_admin: bool,
}

/// Retrieve a party by id.
#[derive(Clone)]
pub struct GetParty {
    repo: Arc<dyn PartyRepository>,
}

impl GetParty {
    pub fn new(repo: Arc<dyn PartyRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self, query), fields(party_id = %id))]
    pub async fn execute(
        &self,
        id: Uuid,
        query: GetPartyQuery,
    ) -> Result<PartyResult, ApplicationError> {
        let party = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;

        if !query.is_admin
            && !self
                .repo
                .is_user_member_of_party(query.actor_user_id, id)
                .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        Ok(map_party_to_result(party))
    }
}
