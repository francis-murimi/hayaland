use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScoreResult {
    pub trust_score_id: Uuid,
    pub party_id: Uuid,
    pub party_name: String,
    pub overall_score: f64,
    pub score_out_of_5: f64,
    pub tier: String,
    pub role_scores: RoleScoresResult,
    pub detailed_metrics: DetailedMetrics,
    pub component_breakdown: HashMap<String, f64>,
    pub last_calculated_at: Option<time::OffsetDateTime>,
    pub next_calculation_at: Option<time::OffsetDateTime>,
    pub calculation_formula: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleScoresResult {
    pub as_supplier: Option<RoleScoreResult>,
    pub as_consumer: Option<RoleScoreResult>,
    pub as_enhancer: Option<RoleScoreResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleScoreResult {
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedMetrics {
    pub deals_completed_count: i64,
    pub deals_cancelled_count: i64,
    pub deals_disputed_count: i64,
    pub timeouts_count: i64,
    pub no_shows_count: i64,
    pub completion_rate: f64,
    pub average_response_hours: Option<f64>,
    pub profile_completeness: f64,
    pub verification_level: i32,
    pub longevity_days: i64,
    pub total_reviews: usize,
    pub average_rating: Option<f64>,
}
