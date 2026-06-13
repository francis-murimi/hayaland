use domain::entities::{DealRole, PartyType, RoleProfile, VerificationStatus};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Command to create a new party.
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePartyCommand {
    pub actor_user_id: Uuid,
    pub party_type: PartyType,
    pub display_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub tax_id: Option<String>,
    pub primary_domain_id: Option<Uuid>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<f64>,
    pub roles: Vec<DealRole>,
}

/// Command to update a party.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePartyCommand {
    pub actor_user_id: Uuid,
    pub is_admin: bool,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub tax_id: Option<String>,
    pub primary_domain_id: Option<Uuid>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<f64>,
    pub verification_status: Option<VerificationStatus>,
    pub is_active: Option<bool>,
}

/// Command to add a role to a party.
#[derive(Debug, Clone, Deserialize)]
pub struct AddPartyRoleCommand {
    pub actor_user_id: Uuid,
    pub is_admin: bool,
    pub role: DealRole,
    pub profile: RoleProfile,
}

/// Query parameters for searching parties.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SearchPartiesQuery {
    pub query: Option<String>,
    pub roles: Vec<DealRole>,
    pub party_types: Vec<PartyType>,
    pub verification_statuses: Vec<VerificationStatus>,
    pub min_trust_score: Option<f64>,
    pub max_trust_score: Option<f64>,
    pub primary_domain_id: Option<Uuid>,
    pub active_only: Option<bool>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub radius_km: Option<f64>,
    pub limit: i64,
    pub offset: i64,
}

/// Full party representation returned by use cases.
#[derive(Debug, Clone, Serialize)]
pub struct PartyResult {
    pub id: Uuid,
    pub party_type: PartyType,
    pub display_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub tax_id: Option<String>,
    pub verification_status: VerificationStatus,
    pub primary_domain_id: Option<Uuid>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<f64>,
    pub trust_score: f64,
    pub total_deals_completed: i32,
    pub total_deals_initiated: i32,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Lightweight party summary for lists.
#[derive(Debug, Clone, Serialize)]
pub struct PartySummaryResult {
    pub id: Uuid,
    pub party_type: PartyType,
    pub display_name: String,
    pub email: String,
    pub verification_status: VerificationStatus,
    pub primary_domain_id: Option<Uuid>,
    pub trust_score: f64,
    pub is_active: bool,
}

/// Result of a party search.
#[derive(Debug, Clone, Serialize)]
pub struct SearchPartiesResult {
    pub parties: Vec<PartySummaryResult>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Role representation returned by use cases.
#[derive(Debug, Clone, Serialize)]
pub struct RoleResult {
    pub role_type: DealRole,
    pub profile: RoleProfile,
    pub is_active: bool,
    pub assigned_at: OffsetDateTime,
}
