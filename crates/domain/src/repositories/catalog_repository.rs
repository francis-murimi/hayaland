use async_trait::async_trait;
use uuid::Uuid;

use crate::entities::{Enhancement, Need, Resource};
use crate::errors::DomainError;

/// Discriminator for the three catalogue item types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogItemType {
    Resource,
    Need,
    Enhancement,
}

impl CatalogItemType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CatalogItemType::Resource => "RESOURCE",
            CatalogItemType::Need => "NEED",
            CatalogItemType::Enhancement => "ENHANCEMENT",
        }
    }
}

impl TryFrom<&str> for CatalogItemType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "RESOURCE" => Ok(CatalogItemType::Resource),
            "NEED" => Ok(CatalogItemType::Need),
            "ENHANCEMENT" => Ok(CatalogItemType::Enhancement),
            _ => Err(DomainError::InvalidCatalogSearchParameters {
                message: format!("unknown catalog item type: {value}"),
            }),
        }
    }
}

/// Geo search parameters.
#[derive(Debug, Clone, Default)]
pub struct GeoSearch {
    pub latitude: f64,
    pub longitude: f64,
    pub radius_km: f64,
}

/// Search criteria for catalogue listings.
#[derive(Debug, Clone, Default)]
pub struct CatalogSearchCriteria {
    pub party_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub domain_category_id: Option<Uuid>,
    pub query: Option<String>,
    pub status: Option<CatalogItemStatus>,
    pub geo: Option<GeoSearch>,
    pub verified_only: bool,
    pub featured_only: bool,
    pub include_hidden: bool,
    pub include_inactive: bool,
    pub sort: CatalogSort,
    pub limit: i64,
    pub offset: i64,
}

/// Status filter for catalogue listings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogItemStatus {
    Active,
    Inactive,
    All,
}

impl CatalogItemStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CatalogItemStatus::Active => "ACTIVE",
            CatalogItemStatus::Inactive => "INACTIVE",
            CatalogItemStatus::All => "ALL",
        }
    }
}

impl TryFrom<&str> for CatalogItemStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "ACTIVE" => Ok(CatalogItemStatus::Active),
            "INACTIVE" => Ok(CatalogItemStatus::Inactive),
            "ALL" => Ok(CatalogItemStatus::All),
            _ => Err(DomainError::InvalidCatalogSearchParameters {
                message: format!("unknown catalog item status: {value}"),
            }),
        }
    }
}

/// Sort order for catalogue listings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatalogSort {
    #[default]
    Newest,
    TrustScore,
    Nearest,
    Relevance,
}

impl CatalogSort {
    pub fn as_str(&self) -> &'static str {
        match self {
            CatalogSort::Newest => "NEWEST",
            CatalogSort::TrustScore => "TRUST_SCORE",
            CatalogSort::Nearest => "NEAREST",
            CatalogSort::Relevance => "RELEVANCE",
        }
    }
}

impl TryFrom<&str> for CatalogSort {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "NEWEST" => Ok(CatalogSort::Newest),
            "TRUST_SCORE" => Ok(CatalogSort::TrustScore),
            "NEAREST" => Ok(CatalogSort::Nearest),
            "RELEVANCE" => Ok(CatalogSort::Relevance),
            _ => Err(DomainError::InvalidCatalogSearchParameters {
                message: format!("unknown catalog sort: {value}"),
            }),
        }
    }
}

/// Admin flags that can be updated by platform moderators.
#[derive(Debug, Clone, Default)]
pub struct AdminFlags {
    pub platform_hidden: Option<bool>,
    pub platform_featured: Option<bool>,
    pub admin_notes: Option<String>,
    pub admin_reviewed_by: Option<Uuid>,
}

/// Paginated result of a catalogue search.
#[derive(Debug, Clone)]
pub struct CatalogListResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[async_trait]
pub trait CatalogRepository: Send + Sync {
    // Resources
    async fn create_resource(&self, resource: &Resource) -> Result<(), DomainError>;
    async fn update_resource(&self, resource: &Resource) -> Result<(), DomainError>;
    async fn delete_resource(&self, id: Uuid) -> Result<(), DomainError>;
    async fn find_resource_by_id(&self, id: Uuid) -> Result<Option<Resource>, DomainError>;
    async fn list_resources(
        &self,
        criteria: &CatalogSearchCriteria,
    ) -> Result<CatalogListResult<Resource>, DomainError>;
    async fn count_resources_for_party(&self, party_id: Uuid) -> Result<i64, DomainError>;

    // Needs
    async fn create_need(&self, need: &Need) -> Result<(), DomainError>;
    async fn update_need(&self, need: &Need) -> Result<(), DomainError>;
    async fn delete_need(&self, id: Uuid) -> Result<(), DomainError>;
    async fn find_need_by_id(&self, id: Uuid) -> Result<Option<Need>, DomainError>;
    async fn list_needs(
        &self,
        criteria: &CatalogSearchCriteria,
    ) -> Result<CatalogListResult<Need>, DomainError>;
    async fn count_needs_for_party(&self, party_id: Uuid) -> Result<i64, DomainError>;

    // Enhancements
    async fn create_enhancement(&self, enhancement: &Enhancement) -> Result<(), DomainError>;
    async fn update_enhancement(&self, enhancement: &Enhancement) -> Result<(), DomainError>;
    async fn delete_enhancement(&self, id: Uuid) -> Result<(), DomainError>;
    async fn find_enhancement_by_id(&self, id: Uuid) -> Result<Option<Enhancement>, DomainError>;
    async fn list_enhancements(
        &self,
        criteria: &CatalogSearchCriteria,
    ) -> Result<CatalogListResult<Enhancement>, DomainError>;
    async fn count_enhancements_for_party(&self, party_id: Uuid) -> Result<i64, DomainError>;

    // Admin
    async fn update_resource_admin_flags(
        &self,
        id: Uuid,
        flags: AdminFlags,
    ) -> Result<(), DomainError>;
    async fn update_need_admin_flags(&self, id: Uuid, flags: AdminFlags)
        -> Result<(), DomainError>;
    async fn update_enhancement_admin_flags(
        &self,
        id: Uuid,
        flags: AdminFlags,
    ) -> Result<(), DomainError>;

    // Deal binding helpers
    async fn increment_deal_count(
        &self,
        item_type: CatalogItemType,
        id: Uuid,
    ) -> Result<(), DomainError>;
    async fn find_resources_by_deal(&self, deal_id: Uuid) -> Result<Vec<Resource>, DomainError>;
    async fn find_needs_by_deal(&self, deal_id: Uuid) -> Result<Vec<Need>, DomainError>;
    async fn find_enhancements_by_deal(
        &self,
        deal_id: Uuid,
    ) -> Result<Vec<Enhancement>, DomainError>;

    // Category counts
    async fn count_active_items_by_category(
        &self,
        category_id: Uuid,
    ) -> Result<CategoryItemCounts, DomainError>;
}

/// Counts of active catalogue items for a category.
#[derive(Debug, Clone, Default)]
pub struct CategoryItemCounts {
    pub resource_count: i64,
    pub need_count: i64,
    pub enhancement_count: i64,
}

/// Input for binding a catalogue item to a deal.
#[derive(Debug, Clone)]
pub struct BindCatalogItemInput {
    pub catalog_item_id: Uuid,
    pub deal_id: Uuid,
    pub overrides: Option<serde_json::Value>,
}
