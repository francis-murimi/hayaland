pub mod dto;

use crate::errors::ApplicationError;
use crate::search::dto::{SearchQuery, SearchResultDto, SearchResultItemDto};
use domain::repositories::{SearchRepository, SearchResult, SearchResultItem, SearchTarget};
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct Search {
    repo: Arc<dyn SearchRepository>,
}

impl Search {
    pub fn new(repo: Arc<dyn SearchRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, query: SearchQuery) -> Result<SearchResultDto, ApplicationError> {
        let target = query
            .r#type
            .as_deref()
            .map(SearchTarget::try_from)
            .transpose()
            .map_err(ApplicationError::from)?
            .unwrap_or(SearchTarget::Resource);

        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        let offset = query.offset.unwrap_or(0).max(0);

        let result = self.repo.search(&query.q, target, limit, offset).await?;
        info!(target = %target.as_str(), query = %query.q, total = %result.total, "searched");
        Ok(map_search_result(result))
    }
}

fn map_search_result(result: SearchResult) -> SearchResultDto {
    SearchResultDto {
        items: result.items.into_iter().map(map_search_item).collect(),
        total: result.total,
        limit: result.limit,
        offset: result.offset,
    }
}

fn map_search_item(item: SearchResultItem) -> SearchResultItemDto {
    match item {
        SearchResultItem::Party(p) => SearchResultItemDto::Party {
            id: p.id,
            display_name: p.display_name,
            party_type: p.party_type,
            verification_status: p.verification_status,
            trust_score: p.trust_score,
            primary_domain_id: p.primary_domain_id,
            is_active: p.is_active,
            created_at: p.created_at,
        },
        SearchResultItem::Resource(c) => SearchResultItemDto::Resource {
            id: c.id,
            owner_party_id: c.owner_party_id,
            category_id: c.category_id,
            title: c.title,
            description: c.description,
            is_active: c.is_active,
            verified_by_platform: c.verified_by_platform,
            created_at: c.created_at,
        },
        SearchResultItem::Need(c) => SearchResultItemDto::Need {
            id: c.id,
            owner_party_id: c.owner_party_id,
            category_id: c.category_id,
            title: c.title,
            description: c.description,
            is_active: c.is_active,
            verified_by_platform: c.verified_by_platform,
            created_at: c.created_at,
        },
        SearchResultItem::Enhancement(c) => SearchResultItemDto::Enhancement {
            id: c.id,
            owner_party_id: c.owner_party_id,
            category_id: c.category_id,
            title: c.title,
            description: c.description,
            is_active: c.is_active,
            verified_by_platform: c.verified_by_platform,
            created_at: c.created_at,
        },
        SearchResultItem::Deal(d) => SearchResultItemDto::Deal {
            id: d.id,
            deal_reference: d.deal_reference,
            deal_title: d.deal_title,
            deal_description: d.deal_description,
            deal_status: d.deal_status,
            domain_category_id: d.domain_category_id,
            initiator_party_id: d.initiator_party_id,
            is_public: d.is_public,
            created_at: d.created_at,
        },
    }
}
