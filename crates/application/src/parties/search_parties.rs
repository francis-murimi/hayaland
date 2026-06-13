use crate::errors::ApplicationError;
use crate::parties::dto::{PartySummaryResult, SearchPartiesQuery, SearchPartiesResult};
use domain::repositories::{PartyRepository, PartySearchCriteria};
use std::sync::Arc;

/// Search and filter parties across the platform.
#[derive(Clone)]
pub struct SearchParties {
    repo: Arc<dyn PartyRepository>,
}

impl SearchParties {
    pub fn new(repo: Arc<dyn PartyRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        query: SearchPartiesQuery,
    ) -> Result<SearchPartiesResult, ApplicationError> {
        let criteria = PartySearchCriteria {
            query: query.query,
            roles: query.roles,
            party_types: query.party_types,
            verification_statuses: query.verification_statuses,
            min_trust_score: query.min_trust_score,
            max_trust_score: query.max_trust_score,
            primary_domain_id: query.primary_domain_id,
            active_only: query.active_only,
            latitude: query.latitude,
            longitude: query.longitude,
            radius_km: query.radius_km,
            limit: query.limit.clamp(1, 100),
            offset: query.offset.max(0),
        };

        let parties = self.repo.list(&criteria).await?;
        let total = self.repo.count(&criteria).await?;

        let results = parties
            .into_iter()
            .map(|party| PartySummaryResult {
                id: party.id,
                party_type: party.party_type,
                display_name: party.display_name.as_str().to_owned(),
                email: party.email.as_str().to_owned(),
                verification_status: party.verification_status,
                primary_domain_id: party.primary_domain_id,
                trust_score: party.trust_score,
                is_active: party.is_active,
            })
            .collect();

        Ok(SearchPartiesResult {
            parties: results,
            total,
            limit: criteria.limit,
            offset: criteria.offset,
        })
    }
}
