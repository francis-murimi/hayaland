use crate::errors::ApplicationError;
use crate::parties::create_party::map_party_to_result;
use crate::parties::dto::PartyResult;
use domain::repositories::PartyRepository;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

/// Retrieve a party by id.
#[derive(Clone)]
pub struct GetParty {
    repo: Arc<dyn PartyRepository>,
}

impl GetParty {
    pub fn new(repo: Arc<dyn PartyRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self), fields(party_id = %id))]
    pub async fn execute(&self, id: Uuid) -> Result<PartyResult, ApplicationError> {
        let party = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;
        Ok(map_party_to_result(party))
    }
}
