// ============================================================================
// Catalogue DTOs
// ============================================================================

use application::catalog::dto::{
    EnhancementPublicResult, EnhancementResult, EnhancementSummaryResult, NeedPublicResult,
    NeedResult, NeedSummaryResult, ResourcePublicResult, ResourceResult, ResourceSummaryResult,
};
use application::catalog::AdminCatalogItemResult;
use domain::entities::{NeedPriority, ResourceCondition};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;
use validator::Validate;

// ---------------------------------------------------------------------------
// Resource DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateResourceRequest {
    pub resource_type_id: Uuid,
    #[validate(length(min = 3, max = 200, message = "resource name must be 3-200 characters"))]
    pub resource_name: String,
    pub description: Option<String>,
    pub condition: Option<ResourceCondition>,
    #[serde(default)]
    pub quantity: Decimal,
    #[validate(length(min = 1, max = 50, message = "quantity unit must be 1-50 characters"))]
    pub quantity_unit: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_address: Option<serde_json::Value>,
    pub availability_start: Option<Date>,
    pub availability_end: Option<Date>,
    #[serde(default)]
    pub document_urls: Vec<String>,
    pub opportunity_cost: Option<Decimal>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResourceRequest {
    pub resource_type_id: Option<Uuid>,
    #[validate(length(min = 3, max = 200, message = "resource name must be 3-200 characters"))]
    pub resource_name: Option<String>,
    pub description: Option<String>,
    pub condition: Option<ResourceCondition>,
    pub quantity: Option<Decimal>,
    #[validate(length(min = 1, max = 50, message = "quantity unit must be 1-50 characters"))]
    pub quantity_unit: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_address: Option<serde_json::Value>,
    pub availability_start: Option<Date>,
    pub availability_end: Option<Date>,
    pub document_urls: Option<Vec<String>>,
    pub opportunity_cost: Option<Decimal>,
    pub metadata: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ResourceResponse {
    Owner(ResourceResult),
    Public(ResourcePublicResult),
}

impl From<ResourceResult> for ResourceResponse {
    fn from(result: ResourceResult) -> Self {
        Self::Owner(result)
    }
}

#[derive(Debug, Serialize)]
pub struct ResourceSummaryResponse {
    #[serde(flatten)]
    pub summary: ResourceSummaryResult,
}

impl From<ResourceSummaryResult> for ResourceSummaryResponse {
    fn from(summary: ResourceSummaryResult) -> Self {
        Self { summary }
    }
}

// ---------------------------------------------------------------------------
// Need DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateNeedRequest {
    pub need_category_id: Uuid,
    #[validate(length(
        min = 10,
        max = 1000,
        message = "description must be 10-1000 characters"
    ))]
    pub need_description: String,
    #[serde(default)]
    pub required_quantity: Decimal,
    #[validate(length(min = 1, max = 50, message = "quantity unit must be 1-50 characters"))]
    pub quantity_unit: String,
    pub quality_requirements: Option<String>,
    pub required_by_date: Option<Date>,
    pub max_budget: Option<Decimal>,
    pub budget_currency: Option<String>,
    pub estimated_fulfillment_value: Option<Decimal>,
    pub acceptable_variants: Option<String>,
    pub priority: Option<NeedPriority>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_address: Option<serde_json::Value>,
    pub delivery_preferences: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNeedRequest {
    pub need_category_id: Option<Uuid>,
    #[validate(length(
        min = 10,
        max = 1000,
        message = "description must be 10-1000 characters"
    ))]
    pub need_description: Option<String>,
    pub required_quantity: Option<Decimal>,
    #[validate(length(min = 1, max = 50, message = "quantity unit must be 1-50 characters"))]
    pub quantity_unit: Option<String>,
    pub quality_requirements: Option<String>,
    pub required_by_date: Option<Date>,
    pub max_budget: Option<Decimal>,
    pub budget_currency: Option<String>,
    pub estimated_fulfillment_value: Option<Decimal>,
    pub acceptable_variants: Option<String>,
    pub priority: Option<NeedPriority>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_address: Option<serde_json::Value>,
    pub delivery_preferences: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum NeedResponse {
    Owner(NeedResult),
    Public(NeedPublicResult),
}

impl From<NeedResult> for NeedResponse {
    fn from(result: NeedResult) -> Self {
        Self::Owner(result)
    }
}

#[derive(Debug, Serialize)]
pub struct NeedSummaryResponse {
    #[serde(flatten)]
    pub summary: NeedSummaryResult,
}

impl From<NeedSummaryResult> for NeedSummaryResponse {
    fn from(summary: NeedSummaryResult) -> Self {
        Self { summary }
    }
}

// ---------------------------------------------------------------------------
// Enhancement DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateEnhancementRequest {
    pub enhancement_type_id: Uuid,
    #[validate(length(
        min = 3,
        max = 200,
        message = "enhancement name must be 3-200 characters"
    ))]
    pub enhancement_name: String,
    pub description: Option<String>,
    pub input_quantity: Option<Decimal>,
    pub quantity_unit: Option<String>,
    pub estimated_input_cost: Option<Decimal>,
    pub service_duration_hours: Option<Decimal>,
    pub estimated_completion_days: Option<i32>,
    pub deliverables: Option<String>,
    pub prerequisites: Option<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    pub certifications: Option<serde_json::Value>,
    #[serde(default)]
    pub equipment: Vec<String>,
    pub pricing: Option<serde_json::Value>,
    pub availability: Option<serde_json::Value>,
    pub service_area: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEnhancementRequest {
    pub enhancement_type_id: Option<Uuid>,
    #[validate(length(
        min = 3,
        max = 200,
        message = "enhancement name must be 3-200 characters"
    ))]
    pub enhancement_name: Option<String>,
    pub description: Option<String>,
    pub input_quantity: Option<Decimal>,
    pub quantity_unit: Option<String>,
    pub estimated_input_cost: Option<Decimal>,
    pub service_duration_hours: Option<Decimal>,
    pub estimated_completion_days: Option<i32>,
    pub deliverables: Option<String>,
    pub prerequisites: Option<String>,
    pub skills: Option<Vec<String>>,
    pub certifications: Option<serde_json::Value>,
    pub equipment: Option<Vec<String>>,
    pub pricing: Option<serde_json::Value>,
    pub availability: Option<serde_json::Value>,
    pub service_area: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum EnhancementResponse {
    Owner(EnhancementResult),
    Public(EnhancementPublicResult),
}

impl From<EnhancementResult> for EnhancementResponse {
    fn from(result: EnhancementResult) -> Self {
        Self::Owner(result)
    }
}

#[derive(Debug, Serialize)]
pub struct EnhancementSummaryResponse {
    #[serde(flatten)]
    pub summary: EnhancementSummaryResult,
}

impl From<EnhancementSummaryResult> for EnhancementSummaryResponse {
    fn from(summary: EnhancementSummaryResult) -> Self {
        Self { summary }
    }
}

// ---------------------------------------------------------------------------
// Search / admin / contact / settings DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Validate, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct CatalogSearchQueryParams {
    pub text: Option<String>,
    pub category_id: Option<Uuid>,
    pub domain_category_id: Option<Uuid>,
    pub party_id: Option<Uuid>,
    #[serde(rename = "lat")]
    pub latitude: Option<f64>,
    #[serde(rename = "lng")]
    pub longitude: Option<f64>,
    #[serde(rename = "radiusKm")]
    pub radius_km: Option<f64>,
    pub status: Option<String>,
    pub verified_only: Option<bool>,
    pub featured_only: Option<bool>,
    pub sort: Option<String>,
    #[validate(range(min = 1, max = 1000, message = "limit must be between 1 and 1000"))]
    pub limit: Option<i64>,
    #[validate(range(min = 0, message = "offset must be at least 0"))]
    pub offset: Option<i64>,
}

impl From<CatalogSearchQueryParams> for application::catalog::dto::CatalogSearchQuery {
    fn from(params: CatalogSearchQueryParams) -> Self {
        Self {
            text: params.text,
            category_id: params.category_id,
            domain_category_id: params.domain_category_id,
            party_id: params.party_id,
            latitude: params.latitude,
            longitude: params.longitude,
            radius_km: params.radius_km,
            status: params.status,
            verified_only: params.verified_only.unwrap_or(false),
            featured_only: params.featured_only.unwrap_or(false),
            sort: params.sort,
            limit: params.limit.unwrap_or(Self::default_limit()),
            offset: params.offset.unwrap_or(0),
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AdminUpdateCatalogFlagsRequest {
    pub platform_hidden: Option<bool>,
    pub platform_featured: Option<bool>,
    pub admin_notes: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum AdminCatalogItemResponse {
    Resource(ResourceResult),
    Need(NeedResult),
    Enhancement(EnhancementResult),
}

impl From<AdminCatalogItemResult> for AdminCatalogItemResponse {
    fn from(result: AdminCatalogItemResult) -> Self {
        match result {
            AdminCatalogItemResult::Resource(r) => Self::Resource(r),
            AdminCatalogItemResult::Need(n) => Self::Need(n),
            AdminCatalogItemResult::Enhancement(e) => Self::Enhancement(e),
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ContactCatalogOwnerRequest {
    #[validate(length(min = 1, max = 4000, message = "message cannot be empty"))]
    pub message: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePartyCatalogSettingsRequest {
    pub accepts_catalog_inquiries: Option<bool>,
    pub public_contact_email: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDealResourceRequest {
    pub item_id: Uuid,
    #[serde(flatten)]
    #[validate(nested)]
    pub update: UpdateResourceRequest,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct BindCatalogItemRequest {
    pub item_id: Uuid,
    pub overrides: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Category / discovery DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryNode {
    pub id: Uuid,
    pub parent_category_id: Option<Uuid>,
    pub category_name: String,
    pub category_code: String,
    pub description: Option<String>,
    pub category_type: String,
    pub icon_url: Option<String>,
    pub children: Vec<CategoryNode>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTreeResponse {
    pub categories: Vec<CategoryNode>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryDomainResponse {
    pub id: Uuid,
    pub category_code: String,
    pub category_name: String,
    pub description: Option<String>,
    pub resource_count: i64,
    pub need_count: i64,
    pub enhancement_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryDomainsResponse {
    pub domains: Vec<DiscoveryDomainResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryDomainChild {
    pub id: Uuid,
    pub parent_category_id: Option<Uuid>,
    pub category_code: String,
    pub category_name: String,
    pub category_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryDomainDetailResponse {
    pub id: Uuid,
    pub category_code: String,
    pub category_name: String,
    pub description: Option<String>,
    pub resource_count: i64,
    pub need_count: i64,
    pub enhancement_count: i64,
    pub child_categories: Vec<DiscoveryDomainChild>,
}
