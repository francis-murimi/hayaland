use domain::entities::{DealRole, MatchScoreWeights, MatchStatus};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Command to generate match suggestions for a party or the whole platform.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateMatchesCommand {
    #[serde(default)]
    pub actor_user_id: Uuid,
    #[serde(default)]
    pub actor_party_id: Option<Uuid>,
    #[serde(default)]
    pub is_admin: bool,
    pub min_score: Option<f64>,
    pub max_suggestions: Option<usize>,
    pub weights: Option<MatchScoreWeights>,
}

/// Command to respond to a match suggestion.
#[derive(Debug, Clone, Deserialize)]
pub struct RespondToMatchCommand {
    #[serde(default)]
    pub actor_user_id: Uuid,
    #[serde(default)]
    pub actor_party_id: Uuid,
    pub match_suggestion_id: Uuid,
    pub response: MatchResponseAction,
    pub notes: Option<String>,
    pub counter_value: Option<Decimal>,
}

/// Possible responses to a match suggestion.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MatchResponseAction {
    Accept,
    Decline,
    CounterPropose,
}

/// Query for listing match suggestions visible to a party.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ListMatchesQuery {
    pub party_id: Option<Uuid>,
    pub role: Option<DealRole>,
    pub status: Option<MatchStatus>,
    pub min_score: Option<f64>,
    pub max_score: Option<f64>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for ListMatchesQuery {
    fn default() -> Self {
        Self {
            party_id: None,
            role: None,
            status: None,
            min_score: None,
            max_score: None,
            limit: 20,
            offset: 0,
        }
    }
}

/// Full match suggestion representation returned by use cases.
#[derive(Debug, Clone, Serialize)]
pub struct MatchSuggestionResult {
    pub id: Uuid,
    pub supplier_party_id: Uuid,
    pub consumer_party_id: Uuid,
    pub enhancer_party_id: Uuid,
    pub match_status: MatchStatus,
    pub match_score: f64,
    pub score_breakdown: domain::entities::MatchScoreBreakdown,
    pub match_reason: String,
    pub resource_category_id: Option<Uuid>,
    pub need_category_id: Option<Uuid>,
    pub enhancement_category_id: Option<Uuid>,
    pub suggested_deal_value: Option<Decimal>,
    pub generated_by: domain::entities::MatchGeneratedBy,
    pub expires_at: Option<OffsetDateTime>,
    pub converted_deal_id: Option<Uuid>,
    pub counter_notes: Option<String>,
    pub responded_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Status counts for a party or the platform.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MatchStatusCountsResult {
    pub pending: i64,
    pub accepted: i64,
    pub declined: i64,
    pub counter_proposed: i64,
    pub expired: i64,
    pub converted_to_deal: i64,
}

impl From<domain::repositories::MatchCountByStatus> for MatchStatusCountsResult {
    fn from(counts: domain::repositories::MatchCountByStatus) -> Self {
        Self {
            pending: counts.pending,
            accepted: counts.accepted,
            declined: counts.declined,
            counter_proposed: counts.counter_proposed,
            expired: counts.expired,
            converted_to_deal: counts.converted_to_deal,
        }
    }
}

/// Command for an admin to mutate a match suggestion.
#[derive(Debug, Clone, Deserialize)]
pub struct AdminUpdateMatchCommand {
    #[serde(default)]
    pub admin_user_id: Uuid,
    pub match_suggestion_id: Uuid,
    pub new_status: MatchStatus,
    pub reason: Option<String>,
}

/// Command for an admin to delete suggestions for a party.
#[derive(Debug, Clone, Deserialize)]
pub struct AdminDeleteMatchesCommand {
    #[serde(default)]
    pub admin_user_id: Uuid,
    pub party_id: Uuid,
    pub status: Option<MatchStatus>,
}
