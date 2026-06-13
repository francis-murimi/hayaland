use domain::entities::{DealRole, DealStatus, PaymentScheduleEntry, TermStatus, TermType};
use domain::services::{PartyFeedback, ValidationIssue};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Command to create a new draft deal.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDealCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub domain_category_id: Uuid,
    pub consumer_party_id: Uuid,
    pub enhancer_party_id: Uuid,
    pub expected_start_date: Option<time::Date>,
    pub expected_end_date: Option<time::Date>,
    pub timeline: Option<serde_json::Value>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Command to update a draft deal.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateDealCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub title: Option<String>,
    pub description: Option<String>,
    pub domain_category_id: Option<Uuid>,
    pub expected_start_date: Option<time::Date>,
    pub expected_end_date: Option<time::Date>,
    pub timeline: Option<serde_json::Value>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Command to submit a draft deal to the suggested state.
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitDealCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
}

/// Command to execute a deal state transition.
#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteTransitionCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub new_status: DealStatus,
    pub reason: Option<String>,
    #[serde(default)]
    pub acknowledge_warnings: bool,
}

/// Full deal representation returned by use cases.
#[derive(Debug, Clone, Serialize)]
pub struct DealResult {
    pub id: Uuid,
    pub deal_reference: String,
    pub title: String,
    pub description: Option<String>,
    pub domain_category_id: Uuid,
    pub initiator_party_id: Uuid,
    pub initiator_role: DealRole,
    pub deal_status: DealStatus,
    pub expected_start_date: Option<time::Date>,
    pub expected_end_date: Option<time::Date>,
    pub actual_start_date: Option<time::Date>,
    pub actual_end_date: Option<time::Date>,
    pub timeline: Option<serde_json::Value>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub total_deal_value: Option<Decimal>,
    pub currency: String,
    pub platform_fee_percentage: Decimal,
    pub platform_fee_amount: Decimal,
    pub win_win_win_validated: bool,
    pub validation_score: Option<Decimal>,
    pub is_public: bool,
    pub current_state_entered_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub participations: Vec<DealParticipationResult>,
}

/// Participation representation returned with a deal.
#[derive(Debug, Clone, Serialize)]
pub struct DealParticipationResult {
    pub id: Uuid,
    pub party_id: Uuid,
    pub role: DealRole,
    pub participation_status: String,
    pub is_initiator: bool,
    pub value_share_percentage: Option<Decimal>,
    pub value_share_amount: Option<Decimal>,
    pub invited_at: Option<OffsetDateTime>,
    pub responded_at: Option<OffsetDateTime>,
}

/// Result of a deal list.
#[derive(Debug, Clone, Serialize)]
pub struct DealListResult {
    pub deals: Vec<DealSummaryResult>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Lightweight deal summary.
#[derive(Debug, Clone, Serialize)]
pub struct DealSummaryResult {
    pub id: Uuid,
    pub deal_reference: String,
    pub title: String,
    pub deal_status: DealStatus,
    pub initiator_party_id: Uuid,
    pub my_role: Option<DealRole>,
    pub total_deal_value: Option<Decimal>,
    pub currency: String,
    pub updated_at: OffsetDateTime,
}

/// Query parameters for listing deals visible to the caller.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListDealsQuery {
    pub status: Option<DealStatus>,
    pub limit: i64,
    pub offset: i64,
}

/// Command to propose a new term.
#[derive(Debug, Clone, Deserialize)]
pub struct ProposeTermCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub term_type: TermType,
    pub term_name: String,
    pub description: String,
    pub is_mandatory: bool,
}

/// Command to counter an existing term.
#[derive(Debug, Clone, Deserialize)]
pub struct CounterTermCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub term_id: Uuid,
    pub description: String,
}

/// Command to accept, reject, or withdraw a term.
#[derive(Debug, Clone, Deserialize)]
pub struct TermActionCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub term_id: Uuid,
}

/// Full term representation returned by use cases.
#[derive(Debug, Clone, Serialize)]
pub struct TermResult {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub proposed_by_party_id: Uuid,
    pub term_type: TermType,
    pub term_name: String,
    pub description: String,
    pub negotiation_status: TermStatus,
    pub parent_term_id: Option<Uuid>,
    pub version: i32,
    pub proposed_at: OffsetDateTime,
    pub resolved_at: Option<OffsetDateTime>,
    pub is_mandatory: bool,
    pub resolution: Option<String>,
}

/// Command to set or replace the value distribution for a deal.
#[derive(Debug, Clone, Deserialize)]
pub struct SetValueDistributionCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub total_value: Decimal,
    pub distribution_model: domain::entities::DistributionModel,
    pub supplier_share_percentage: Decimal,
    pub enhancer_share_percentage: Decimal,
    pub platform_fee_percentage: Decimal,
    pub consumer_cost_percentage: Decimal,
    pub payment_schedule: Vec<PaymentScheduleEntry>,
}

/// Value distribution representation returned by use cases.
#[derive(Debug, Clone, Serialize)]
pub struct ValueDistributionResult {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub total_value: Decimal,
    pub currency: String,
    pub distribution_model: domain::entities::DistributionModel,
    pub supplier_share_percentage: Decimal,
    pub supplier_share_amount: Decimal,
    pub consumer_cost_percentage: Decimal,
    pub consumer_cost_amount: Decimal,
    pub enhancer_share_percentage: Decimal,
    pub enhancer_share_amount: Decimal,
    pub platform_fee_percentage: Decimal,
    pub platform_fee_amount: Decimal,
    pub payment_schedule: Vec<PaymentScheduleEntry>,
    pub win_win_win_score: Option<Decimal>,
}

/// Result of running Win-Win-Win validation on a deal.
#[derive(Debug, Clone, Serialize)]
pub struct ValidateDealResult {
    pub score: Decimal,
    pub status: String,
    pub blocked: bool,
    pub violations: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
    pub party_feedback: std::collections::BTreeMap<DealRole, PartyFeedback>,
}
