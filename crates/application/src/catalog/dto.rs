use domain::entities::{DealRole, NeedPriority, ResourceCondition};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Resource DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreateResourceCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub resource_type_id: Uuid,
    pub resource_name: String,
    pub description: Option<String>,
    pub quantity: Decimal,
    pub quantity_unit: String,
    pub condition: Option<ResourceCondition>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_address: Option<serde_json::Value>,
    pub availability_start: Option<Date>,
    pub availability_end: Option<Date>,
    pub document_urls: Vec<String>,
    pub opportunity_cost: Option<Decimal>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateResourceCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub resource_type_id: Option<Uuid>,
    pub resource_name: Option<String>,
    pub description: Option<String>,
    pub quantity: Option<Decimal>,
    pub quantity_unit: Option<String>,
    pub condition: Option<ResourceCondition>,
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

#[derive(Debug, Clone, Serialize)]
pub struct ResourceResult {
    pub id: Uuid,
    pub supplier_party_id: Uuid,
    pub resource_type_id: Uuid,
    pub resource_name: String,
    pub description: Option<String>,
    pub quantity: Decimal,
    pub quantity_unit: String,
    pub condition: Option<ResourceCondition>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_address: Option<serde_json::Value>,
    pub availability_start: Option<Date>,
    pub availability_end: Option<Date>,
    pub document_urls: Vec<String>,
    pub opportunity_cost: Option<Decimal>,
    pub verified_by_platform: bool,
    pub metadata: Option<serde_json::Value>,
    pub is_active: bool,
    pub deal_count: i32,
    pub platform_hidden: bool,
    pub platform_featured: bool,
    pub admin_notes: Option<String>,
    pub admin_reviewed_at: Option<OffsetDateTime>,
    pub admin_reviewed_by: Option<Uuid>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceSummaryResult {
    pub id: Uuid,
    pub supplier_party_id: Uuid,
    pub resource_name: String,
    pub quantity: Decimal,
    pub quantity_unit: String,
    pub condition: Option<ResourceCondition>,
    pub verified_by_platform: bool,
    pub is_active: bool,
    pub platform_featured: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourcePublicResult {
    pub id: Uuid,
    pub supplier_party_id: Uuid,
    pub resource_type_id: Uuid,
    pub resource_name: String,
    pub description: Option<String>,
    pub quantity: Decimal,
    pub quantity_unit: String,
    pub condition: Option<ResourceCondition>,
    pub availability_start: Option<Date>,
    pub availability_end: Option<Date>,
    pub document_urls: Vec<String>,
    pub verified_by_platform: bool,
    pub metadata: Option<serde_json::Value>,
    pub is_active: bool,
    pub platform_featured: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// ---------------------------------------------------------------------------
// Need DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreateNeedCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub need_category_id: Uuid,
    pub need_description: String,
    pub required_quantity: Decimal,
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

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateNeedCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub need_category_id: Option<Uuid>,
    pub need_description: Option<String>,
    pub required_quantity: Option<Decimal>,
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

#[derive(Debug, Clone, Serialize)]
pub struct NeedResult {
    pub id: Uuid,
    pub consumer_party_id: Uuid,
    pub need_category_id: Uuid,
    pub need_description: String,
    pub required_quantity: Decimal,
    pub quantity_unit: String,
    pub quality_requirements: Option<String>,
    pub required_by_date: Option<Date>,
    pub max_budget: Option<Decimal>,
    pub budget_currency: String,
    pub estimated_fulfillment_value: Option<Decimal>,
    pub acceptable_variants: Option<String>,
    pub priority: Option<NeedPriority>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_address: Option<serde_json::Value>,
    pub delivery_preferences: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub is_active: bool,
    pub deal_count: i32,
    pub platform_hidden: bool,
    pub platform_featured: bool,
    pub admin_notes: Option<String>,
    pub admin_reviewed_at: Option<OffsetDateTime>,
    pub admin_reviewed_by: Option<Uuid>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct NeedSummaryResult {
    pub id: Uuid,
    pub consumer_party_id: Uuid,
    pub need_description: String,
    pub required_quantity: Decimal,
    pub quantity_unit: String,
    pub priority: Option<NeedPriority>,
    pub is_active: bool,
    pub platform_featured: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct NeedPublicResult {
    pub id: Uuid,
    pub consumer_party_id: Uuid,
    pub need_category_id: Uuid,
    pub need_description: String,
    pub required_quantity: Decimal,
    pub quantity_unit: String,
    pub quality_requirements: Option<String>,
    pub required_by_date: Option<Date>,
    pub budget_currency: String,
    pub acceptable_variants: Option<String>,
    pub priority: Option<NeedPriority>,
    pub delivery_preferences: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub is_active: bool,
    pub platform_featured: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// ---------------------------------------------------------------------------
// Enhancement DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreateEnhancementCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub enhancement_type_id: Uuid,
    pub enhancement_name: String,
    pub description: Option<String>,
    pub input_quantity: Option<Decimal>,
    pub quantity_unit: Option<String>,
    pub estimated_input_cost: Option<Decimal>,
    pub service_duration_hours: Option<Decimal>,
    pub estimated_completion_days: Option<i32>,
    pub deliverables: Option<String>,
    pub prerequisites: Option<String>,
    pub skills: Vec<String>,
    pub certifications: Option<serde_json::Value>,
    pub equipment: Vec<String>,
    pub pricing: Option<serde_json::Value>,
    pub availability: Option<serde_json::Value>,
    pub service_area: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateEnhancementCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub enhancement_type_id: Option<Uuid>,
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

#[derive(Debug, Clone, Serialize)]
pub struct EnhancementResult {
    pub id: Uuid,
    pub enhancer_party_id: Uuid,
    pub enhancement_type_id: Uuid,
    pub enhancement_name: String,
    pub description: Option<String>,
    pub input_quantity: Option<Decimal>,
    pub quantity_unit: Option<String>,
    pub estimated_input_cost: Option<Decimal>,
    pub service_duration_hours: Option<Decimal>,
    pub estimated_completion_days: Option<i32>,
    pub deliverables: Option<String>,
    pub prerequisites: Option<String>,
    pub skills: Vec<String>,
    pub certifications: Option<serde_json::Value>,
    pub equipment: Vec<String>,
    pub pricing: Option<serde_json::Value>,
    pub availability: Option<serde_json::Value>,
    pub service_area: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub is_complete: bool,
    pub completed_at: Option<OffsetDateTime>,
    pub is_active: bool,
    pub deal_count: i32,
    pub platform_hidden: bool,
    pub platform_featured: bool,
    pub admin_notes: Option<String>,
    pub admin_reviewed_at: Option<OffsetDateTime>,
    pub admin_reviewed_by: Option<Uuid>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnhancementSummaryResult {
    pub id: Uuid,
    pub enhancer_party_id: Uuid,
    pub enhancement_name: String,
    pub service_duration_hours: Option<Decimal>,
    pub estimated_completion_days: Option<i32>,
    pub is_active: bool,
    pub platform_featured: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnhancementPublicResult {
    pub id: Uuid,
    pub enhancer_party_id: Uuid,
    pub enhancement_type_id: Uuid,
    pub enhancement_name: String,
    pub description: Option<String>,
    pub input_quantity: Option<Decimal>,
    pub quantity_unit: Option<String>,
    pub service_duration_hours: Option<Decimal>,
    pub estimated_completion_days: Option<i32>,
    pub deliverables: Option<String>,
    pub prerequisites: Option<String>,
    pub skills: Vec<String>,
    pub certifications: Option<serde_json::Value>,
    pub equipment: Vec<String>,
    pub availability: Option<serde_json::Value>,
    pub service_area: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub is_complete: bool,
    pub is_active: bool,
    pub platform_featured: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// ---------------------------------------------------------------------------
// Shared search / list / admin / contact / settings DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CatalogSearchQuery {
    pub text: Option<String>,
    pub category_id: Option<Uuid>,
    pub domain_category_id: Option<Uuid>,
    pub party_id: Option<Uuid>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub radius_km: Option<f64>,
    pub status: Option<String>,
    pub verified_only: bool,
    pub featured_only: bool,
    pub sort: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

impl CatalogSearchQuery {
    pub fn default_limit() -> i64 {
        20
    }
}

impl Default for CatalogSearchQuery {
    fn default() -> Self {
        Self {
            text: None,
            category_id: None,
            domain_category_id: None,
            party_id: None,
            latitude: None,
            longitude: None,
            radius_km: None,
            status: None,
            verified_only: false,
            featured_only: false,
            sort: None,
            limit: Self::default_limit(),
            offset: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogListResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteCatalogItemCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminUpdateFlagsCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub platform_hidden: Option<bool>,
    pub platform_featured: Option<bool>,
    pub admin_notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContactCatalogOwnerCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub item_type: String,
    pub item_id: Uuid,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePartyCatalogSettingsCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub accepts_catalog_inquiries: Option<bool>,
    pub public_contact_email: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContactCatalogOwnerResult {
    pub conversation_id: Uuid,
    pub message_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct PartyCatalogSettingsResult {
    pub party_id: Uuid,
    pub accepts_catalog_inquiries: bool,
    pub public_contact_email: bool,
}

// ---------------------------------------------------------------------------
// Deal binding DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct BindCatalogItemToDealCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub item_type: String,
    pub item_id: Uuid,
    pub deal_id: Uuid,
    pub overrides: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DealBoundCatalogItemResult {
    pub item_type: String,
    pub item_id: Uuid,
    pub deal_id: Uuid,
    pub catalog_item_id: Uuid,
}

pub fn deal_role_for_item_type(item_type: &str) -> Option<DealRole> {
    match item_type.to_ascii_uppercase().as_str() {
        "RESOURCE" => Some(DealRole::Supplier),
        "NEED" => Some(DealRole::Consumer),
        "ENHANCEMENT" => Some(DealRole::Enhancer),
        _ => None,
    }
}
