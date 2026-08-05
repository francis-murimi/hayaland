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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::errors::DomainError;
    use domain::repositories::{
        SearchableCatalogItem, SearchableDeal, SearchableParty,
    };
    use time::OffsetDateTime;

    struct FakeSearchRepo {
        result_items: Vec<SearchResultItem>,
        last_query: std::sync::Mutex<(String, SearchTarget, i64, i64)>,
    }

    impl FakeSearchRepo {
        fn new(items: Vec<SearchResultItem>) -> Self {
            Self {
                result_items: items,
                last_query: std::sync::Mutex::new((
                    String::new(),
                    SearchTarget::Resource,
                    0,
                    0,
                )),
            }
        }
    }

    #[async_trait]
    impl SearchRepository for FakeSearchRepo {
        async fn search(
            &self,
            query: &str,
            target: SearchTarget,
            limit: i64,
            offset: i64,
        ) -> Result<SearchResult, DomainError> {
            *self.last_query.lock().unwrap() = (query.to_string(), target, limit, offset);
            let total = self.result_items.len() as i64;
            Ok(SearchResult {
                items: self.result_items.clone(),
                total,
                limit,
                offset,
            })
        }
    }

    fn party_item() -> SearchResultItem {
        SearchResultItem::Party(SearchableParty {
            id: uuid::Uuid::now_v7(),
            display_name: "Alice".to_string(),
            party_type: "INDIVIDUAL".to_string(),
            verification_status: "VERIFIED".to_string(),
            trust_score: 88.0,
            primary_domain_id: None,
            is_active: true,
            created_at: OffsetDateTime::now_utc(),
        })
    }

    fn catalog_item(item_type: &str) -> SearchableCatalogItem {
        SearchableCatalogItem {
            id: uuid::Uuid::now_v7(),
            item_type: item_type.to_string(),
            owner_party_id: uuid::Uuid::now_v7(),
            category_id: uuid::Uuid::now_v7(),
            title: "Tractor".to_string(),
            description: Some("A tractor".to_string()),
            is_active: true,
            verified_by_platform: false,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    fn deal_item() -> SearchResultItem {
        SearchResultItem::Deal(SearchableDeal {
            id: uuid::Uuid::now_v7(),
            deal_reference: "DL-1".to_string(),
            deal_title: "Wheat deal".to_string(),
            deal_description: None,
            deal_status: "DRAFT".to_string(),
            domain_category_id: uuid::Uuid::now_v7(),
            initiator_party_id: uuid::Uuid::now_v7(),
            is_public: true,
            created_at: OffsetDateTime::now_utc(),
        })
    }

    #[tokio::test]
    async fn search_maps_all_item_variants() {
        let items = vec![
            party_item(),
            SearchResultItem::Resource(catalog_item("RESOURCE")),
            SearchResultItem::Need(catalog_item("NEED")),
            SearchResultItem::Enhancement(catalog_item("ENHANCEMENT")),
            deal_item(),
        ];
        let repo = Arc::new(FakeSearchRepo::new(items));
        let uc = Search::new(repo);
        let result = uc
            .execute(SearchQuery {
                q: "tractor".to_string(),
                r#type: Some("RESOURCE".to_string()),
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert_eq!(result.total, 5);
        assert_eq!(result.limit, 10);
        assert_eq!(result.items.len(), 5);
        assert!(matches!(
            result.items[0],
            SearchResultItemDto::Party { .. }
        ));
        assert!(matches!(
            result.items[1],
            SearchResultItemDto::Resource { .. }
        ));
        assert!(matches!(result.items[2], SearchResultItemDto::Need { .. }));
        assert!(matches!(
            result.items[3],
            SearchResultItemDto::Enhancement { .. }
        ));
        assert!(matches!(result.items[4], SearchResultItemDto::Deal { .. }));
    }

    #[tokio::test]
    async fn search_defaults_to_resource_target() {
        let repo = Arc::new(FakeSearchRepo::new(vec![]));
        let uc = Search::new(repo.clone());
        uc.execute(SearchQuery {
            q: "x".to_string(),
            r#type: None,
            limit: None,
            offset: None,
        })
        .await
        .unwrap();
        let (_, target, limit, offset) = repo.last_query.lock().unwrap().clone();
        assert_eq!(target, SearchTarget::Resource);
        assert_eq!(limit, 20);
        assert_eq!(offset, 0);
    }

    #[tokio::test]
    async fn search_parses_each_target() {
        for (input, expected) in [
            ("PARTY", SearchTarget::Party),
            ("resource", SearchTarget::Resource),
            ("NEED", SearchTarget::Need),
            ("enhancement", SearchTarget::Enhancement),
            ("DEAL", SearchTarget::Deal),
        ] {
            let repo = Arc::new(FakeSearchRepo::new(vec![]));
            let uc = Search::new(repo.clone());
            uc.execute(SearchQuery {
                q: "x".to_string(),
                r#type: Some(input.to_string()),
                limit: None,
                offset: None,
            })
            .await
            .unwrap();
            let (_, target, _, _) = repo.last_query.lock().unwrap().clone();
            assert_eq!(target, expected, "input {input}");
        }
    }

    #[tokio::test]
    async fn search_rejects_invalid_target() {
        let uc = Search::new(Arc::new(FakeSearchRepo::new(vec![])));
        let err = uc
            .execute(SearchQuery {
                q: "x".to_string(),
                r#type: Some("bogus".to_string()),
                limit: None,
                offset: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn search_clamps_pagination() {
        let repo = Arc::new(FakeSearchRepo::new(vec![]));
        let uc = Search::new(repo.clone());
        uc.execute(SearchQuery {
            q: "x".to_string(),
            r#type: None,
            limit: Some(0),
            offset: Some(-10),
        })
        .await
        .unwrap();
        let (_, _, limit, offset) = repo.last_query.lock().unwrap().clone();
        assert_eq!(limit, 1);
        assert_eq!(offset, 0);

        uc.execute(SearchQuery {
            q: "x".to_string(),
            r#type: None,
            limit: Some(500),
            offset: Some(5),
        })
        .await
        .unwrap();
        let (_, _, limit, offset) = repo.last_query.lock().unwrap().clone();
        assert_eq!(limit, 100);
        assert_eq!(offset, 5);
    }
}
