use crate::errors::DomainError;
use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

/// The entity type to search over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchTarget {
    Party,
    Resource,
    Need,
    Enhancement,
    Deal,
}

impl SearchTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchTarget::Party => "PARTY",
            SearchTarget::Resource => "RESOURCE",
            SearchTarget::Need => "NEED",
            SearchTarget::Enhancement => "ENHANCEMENT",
            SearchTarget::Deal => "DEAL",
        }
    }
}

impl TryFrom<&str> for SearchTarget {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "PARTY" | "party" => Ok(SearchTarget::Party),
            "RESOURCE" | "resource" => Ok(SearchTarget::Resource),
            "NEED" | "need" => Ok(SearchTarget::Need),
            "ENHANCEMENT" | "enhancement" => Ok(SearchTarget::Enhancement),
            "DEAL" | "deal" => Ok(SearchTarget::Deal),
            _ => Err(DomainError::InvalidSearchTarget {
                message: format!("unknown search target: {value}"),
            }),
        }
    }
}

/// A public projection of a party returned by search.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SearchableParty {
    pub id: Uuid,
    pub display_name: String,
    pub party_type: String,
    pub verification_status: String,
    pub trust_score: f64,
    pub primary_domain_id: Option<Uuid>,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
}

/// A public projection of a catalog item returned by search.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SearchableCatalogItem {
    pub id: Uuid,
    pub item_type: String, // "RESOURCE", "NEED", "ENHANCEMENT"
    pub owner_party_id: Uuid,
    pub category_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub verified_by_platform: bool,
    pub created_at: OffsetDateTime,
}

/// A public projection of a deal returned by search.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SearchableDeal {
    pub id: Uuid,
    pub deal_reference: String,
    pub deal_title: String,
    pub deal_description: Option<String>,
    pub deal_status: String,
    pub domain_category_id: Uuid,
    pub initiator_party_id: Uuid,
    pub is_public: bool,
    pub created_at: OffsetDateTime,
}

/// A unified search result.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum SearchResultItem {
    Party(SearchableParty),
    Resource(SearchableCatalogItem),
    Need(SearchableCatalogItem),
    Enhancement(SearchableCatalogItem),
    Deal(SearchableDeal),
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub items: Vec<SearchResultItem>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Outbound port for full-text search across the platform.
#[async_trait]
pub trait SearchRepository: Send + Sync {
    /// Search a single entity type by free-text query.
    async fn search(
        &self,
        query: &str,
        target: SearchTarget,
        limit: i64,
        offset: i64,
    ) -> Result<SearchResult, DomainError>;
}
