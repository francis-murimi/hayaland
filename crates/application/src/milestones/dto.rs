use domain::entities::{Currency, Milestone};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMilestoneCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub milestone_name: String,
    pub description: Option<String>,
    pub assigned_to_party_id: Uuid,
    pub verified_by_party_id: Uuid,
    pub due_date: Option<time::Date>,
    pub completion_criteria: String,
    pub payment_trigger_amount: Option<Decimal>,
    pub display_order: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMilestoneCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub milestone_id: Uuid,
    pub milestone_name: Option<String>,
    pub description: Option<String>,
    pub assigned_to_party_id: Option<Uuid>,
    pub verified_by_party_id: Option<Uuid>,
    pub due_date: Option<time::Date>,
    pub completion_criteria: Option<String>,
    pub payment_trigger_amount: Option<Decimal>,
    pub display_order: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MilestoneActionCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub milestone_id: Uuid,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListMilestonesQuery {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetDealProgressQuery {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct MilestoneResult {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub milestone_name: String,
    pub description: Option<String>,
    pub assigned_to_party_id: Uuid,
    pub verified_by_party_id: Uuid,
    pub due_date: Option<time::Date>,
    pub completion_criteria: String,
    pub milestone_status: String,
    pub completion_percentage: Decimal,
    pub payment_trigger_amount: Option<Decimal>,
    pub completed_at: Option<OffsetDateTime>,
    pub display_order: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<Milestone> for MilestoneResult {
    fn from(m: Milestone) -> Self {
        Self {
            id: m.id,
            deal_id: m.deal_id,
            milestone_name: m.milestone_name,
            description: m.description,
            assigned_to_party_id: m.assigned_to_party_id,
            verified_by_party_id: m.verified_by_party_id,
            due_date: m.due_date,
            completion_criteria: m.completion_criteria,
            milestone_status: m.milestone_status.as_str().to_string(),
            completion_percentage: m.completion_percentage,
            payment_trigger_amount: m.payment_trigger_amount,
            completed_at: m.completed_at,
            display_order: m.display_order,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MilestoneWithTransactionResult {
    #[serde(flatten)]
    pub milestone: MilestoneResult,
    pub triggered_transaction_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListMilestonesResult {
    pub milestones: Vec<MilestoneResult>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DealProgressResult {
    pub deal_id: Uuid,
    pub total_milestones: i64,
    pub verified_milestones: i64,
    pub completed_milestones: i64,
    pub in_progress_milestones: i64,
    pub missed_milestones: i64,
    pub overall_completion_percentage: Decimal,
    pub currency: Currency,
}
