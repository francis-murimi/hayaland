use crate::errors::ApplicationError;
use crate::matching::discovery_engine::generate_candidates;
use crate::matching::dto::{GenerateMatchesCommand, MatchSuggestionResult};
use domain::entities::{DealRole, MatchGeneratedBy, MatchSuggestion, Party};
use domain::repositories::{
    CatalogItemStatus, CatalogRepository, CatalogSearchCriteria, CatalogSort, MatchRepository,
    PartyRepository, PartySearchCriteria,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Generate algorithmic match suggestions.
#[derive(Clone)]
pub struct GenerateMatches {
    match_repo: Arc<dyn MatchRepository>,
    party_repo: Arc<dyn PartyRepository>,
    catalog_repo: Arc<dyn CatalogRepository>,
}

impl GenerateMatches {
    pub fn new(
        match_repo: Arc<dyn MatchRepository>,
        party_repo: Arc<dyn PartyRepository>,
        catalog_repo: Arc<dyn CatalogRepository>,
    ) -> Self {
        Self {
            match_repo,
            party_repo,
            catalog_repo,
        }
    }

    #[instrument(skip(self, cmd))]
    pub async fn execute(
        &self,
        cmd: GenerateMatchesCommand,
    ) -> Result<Vec<MatchSuggestionResult>, ApplicationError> {
        if !cmd.is_admin && cmd.actor_party_id.is_none() {
            return Err(ApplicationError::Validation(vec![
                "actor party is required".to_string(),
            ]));
        }

        let weights = cmd.weights.unwrap_or_default();
        weights.validate().map_err(ApplicationError::from)?;

        let min_score = cmd.min_score.unwrap_or(0.0).clamp(0.0, 1.0);
        let max_suggestions = cmd.max_suggestions.unwrap_or(50);

        let suppliers = self.fetch_parties_with_role(DealRole::Supplier).await?;
        let consumers = self.fetch_parties_with_role(DealRole::Consumer).await?;
        let enhancers = self.fetch_parties_with_role(DealRole::Enhancer).await?;

        if suppliers.is_empty() || consumers.is_empty() || enhancers.is_empty() {
            return Ok(Vec::new());
        }

        let supplier_data = self.fetch_catalog_for_suppliers(&suppliers).await?;
        let consumer_data = self.fetch_catalog_for_consumers(&consumers).await?;
        let enhancer_data = self.fetch_catalog_for_enhancers(&enhancers).await?;

        let all_parties: Vec<_> = suppliers
            .iter()
            .chain(consumers.iter())
            .chain(enhancers.iter())
            .cloned()
            .collect();
        let trust_scores = self.fetch_trust_scores_for_parties(&all_parties).await?;

        let mut suggestions = generate_candidates(
            &supplier_data,
            &consumer_data,
            &enhancer_data,
            &trust_scores,
            weights,
            min_score,
            max_suggestions,
        )
        .map_err(ApplicationError::from)?;

        // Mark as algorithm-generated and de-duplicate against existing pending triplets.
        let mut persisted = Vec::new();
        for suggestion in &mut suggestions {
            suggestion.generated_by = MatchGeneratedBy::Algorithm;

            let existing = self
                .match_repo
                .find_existing_pending(
                    suggestion.supplier_party_id,
                    suggestion.consumer_party_id,
                    suggestion.enhancer_party_id,
                )
                .await?;

            if existing.is_none() {
                self.match_repo.create(suggestion).await?;
                persisted.push(map_to_result(suggestion.clone()));
            }
        }

        info!(
            actor = %cmd.actor_user_id,
            generated = persisted.len(),
            "generated match suggestions"
        );

        Ok(persisted)
    }

    async fn fetch_parties_with_role(
        &self,
        role: DealRole,
    ) -> Result<Vec<Party>, ApplicationError> {
        let criteria = PartySearchCriteria {
            roles: vec![role],
            active_only: Some(true),
            limit: 1000,
            ..PartySearchCriteria::default()
        };
        self.party_repo.list(&criteria).await.map_err(Into::into)
    }

    async fn fetch_catalog_for_suppliers(
        &self,
        parties: &[Party],
    ) -> Result<Vec<(Party, Vec<domain::entities::Resource>)>, ApplicationError> {
        let mut result = Vec::new();
        for party in parties {
            let criteria = CatalogSearchCriteria {
                party_id: Some(party.id),
                status: Some(CatalogItemStatus::Active),
                sort: CatalogSort::Newest,
                limit: 100,
                ..CatalogSearchCriteria::default()
            };
            let items = self.catalog_repo.list_resources(&criteria).await?;
            result.push((party.clone(), items.items));
        }
        Ok(result)
    }

    async fn fetch_catalog_for_consumers(
        &self,
        parties: &[Party],
    ) -> Result<Vec<(Party, Vec<domain::entities::Need>)>, ApplicationError> {
        let mut result = Vec::new();
        for party in parties {
            let criteria = CatalogSearchCriteria {
                party_id: Some(party.id),
                status: Some(CatalogItemStatus::Active),
                sort: CatalogSort::Newest,
                limit: 100,
                ..CatalogSearchCriteria::default()
            };
            let items = self.catalog_repo.list_needs(&criteria).await?;
            result.push((party.clone(), items.items));
        }
        Ok(result)
    }

    async fn fetch_catalog_for_enhancers(
        &self,
        parties: &[Party],
    ) -> Result<Vec<(Party, Vec<domain::entities::Enhancement>)>, ApplicationError> {
        let mut result = Vec::new();
        for party in parties {
            let criteria = CatalogSearchCriteria {
                party_id: Some(party.id),
                status: Some(CatalogItemStatus::Active),
                sort: CatalogSort::Newest,
                limit: 100,
                ..CatalogSearchCriteria::default()
            };
            let items = self.catalog_repo.list_enhancements(&criteria).await?;
            result.push((party.clone(), items.items));
        }
        Ok(result)
    }

    async fn fetch_trust_scores_for_parties(
        &self,
        parties: &[Party],
    ) -> Result<HashMap<Uuid, f64>, ApplicationError> {
        let mut scores = HashMap::new();
        for party in parties {
            scores.insert(party.id, party.trust_score);
        }
        Ok(scores)
    }
}

pub(crate) fn map_to_result(suggestion: MatchSuggestion) -> MatchSuggestionResult {
    MatchSuggestionResult {
        id: suggestion.id,
        supplier_party_id: suggestion.supplier_party_id,
        consumer_party_id: suggestion.consumer_party_id,
        enhancer_party_id: suggestion.enhancer_party_id,
        match_status: suggestion.match_status,
        match_score: suggestion.match_score,
        score_breakdown: suggestion.score_breakdown,
        match_reason: suggestion.match_reason,
        resource_category_id: suggestion.resource_category_id,
        need_category_id: suggestion.need_category_id,
        enhancement_category_id: suggestion.enhancement_category_id,
        suggested_deal_value: suggestion.suggested_deal_value,
        generated_by: suggestion.generated_by,
        expires_at: suggestion.expires_at,
        converted_deal_id: suggestion.converted_deal_id,
        counter_notes: suggestion.counter_notes,
        responded_at: suggestion.responded_at,
        created_at: suggestion.created_at,
        updated_at: suggestion.updated_at,
    }
}
