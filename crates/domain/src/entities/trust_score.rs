use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScoreRow {
    pub id: Uuid,
    pub party_id: Uuid,
    pub overall_score: f64,
    pub as_supplier_score: Option<f64>,
    pub as_consumer_score: Option<f64>,
    pub as_enhancer_score: Option<f64>,
    pub deals_completed_count: i64,
    pub deals_cancelled_count: i64,
    pub deals_disputed_count: i64,
    pub timeouts_count: i64,
    pub no_shows_count: i64,
    pub total_completed_value: f64,
    pub average_response_hours: Option<f64>,
    pub profile_completeness: f64,
    pub verification_level: i32,
    pub longevity_days: i64,
    pub calculation_formula: serde_json::Value,
    pub last_calculated_at: Option<OffsetDateTime>,
    pub next_calculation_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScoreResult {
    pub party_id: Uuid,
    pub party_name: String,
    pub overall_score: f64,
    pub as_supplier_score: Option<f64>,
    pub as_consumer_score: Option<f64>,
    pub as_enhancer_score: Option<f64>,
    pub tier: TrustTier,
    pub deals_completed_count: i64,
    pub deals_cancelled_count: i64,
    pub deals_disputed_count: i64,
    pub timeouts_count: i64,
    pub no_shows_count: i64,
    pub total_completed_value: f64,
    pub average_response_hours: Option<f64>,
    pub profile_completeness: f64,
    pub verification_level: i32,
    pub longevity_days: i64,
    pub calculation_formula: serde_json::Value,
    pub last_calculated_at: Option<OffsetDateTime>,
    pub next_calculation_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrustTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
}

impl TrustTier {
    pub fn from_score(score: f64, thresholds: &TrustTierThresholds) -> Self {
        if score >= thresholds.platinum {
            Self::Platinum
        } else if score >= thresholds.gold {
            Self::Gold
        } else if score >= thresholds.silver {
            Self::Silver
        } else {
            Self::Bronze
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TrustTierThresholds {
    pub silver: f64,
    pub gold: f64,
    pub platinum: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScoreInputs {
    pub deals_completed_count: i64,
    pub deals_cancelled_count: i64,
    pub deals_disputed_count: i64,
    pub timeouts_count: i64,
    pub no_shows_count: i64,
    pub total_completed_value: f64,
    pub reviews: Vec<ReviewInput>,
    pub disputes: Vec<DisputeInput>,
    pub role_deals: HashMap<String, RoleDealInput>,
    pub role_reviews: HashMap<String, Vec<ReviewInput>>,
    pub response_metrics: ResponseMetrics,
    pub profile_completeness: f64,
    pub verification_level: i32,
    pub longevity_days: i64,
    pub days_since_last_activity: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewInput {
    pub reviewer_party_id: Option<Uuid>,
    pub reviewer_overall_score: f64,
    pub review_score: f64,
    pub deal_value: f64,
    pub created_at: OffsetDateTime,
    pub is_public: bool,
    pub is_hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeInput {
    pub raised_by_party_id: Uuid,
    pub against_party_id: Option<Uuid>,
    pub resolution_type: Option<String>,
    pub resolution_outcome: Option<String>,
    pub created_at: OffsetDateTime,
    pub resolved_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDealInput {
    pub deals_completed_count: i64,
    pub deals_cancelled_count: i64,
    pub total_completed_value: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResponseMetrics {
    pub average_response_hours: Option<f64>,
    pub messages_received_90d: i64,
    pub messages_responded_90d: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustScoreConfig {
    pub weights: TrustScoreWeights,
    pub tiers: TrustTierThresholds,
    pub cold_start: TrustColdStartConfig,
    pub decay: TrustDecayConfig,
    #[serde(default)]
    pub profile_completeness: ProfileCompletenessConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TrustScoreWeights {
    pub transaction_history: f64,
    pub review_ratings: f64,
    pub profile_completeness: f64,
    pub verification_level: f64,
    pub response_rate: f64,
    pub dispute_history: f64,
    pub longevity: f64,
    pub community: f64,
}

impl Default for TrustScoreWeights {
    fn default() -> Self {
        Self {
            transaction_history: 0.25,
            review_ratings: 0.20,
            profile_completeness: 0.10,
            verification_level: 0.15,
            response_rate: 0.10,
            dispute_history: 0.10,
            longevity: 0.05,
            community: 0.05,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TrustColdStartConfig {
    pub global_average_review_score: f64,
    pub min_reviews_before_own_score_dominates: i64,
}

impl Default for TrustColdStartConfig {
    fn default() -> Self {
        Self {
            global_average_review_score: 2.5,
            min_reviews_before_own_score_dominates: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TrustDecayConfig {
    pub inactivity_penalty_per_30_days: f64,
    pub max_inactivity_penalty: f64,
}

impl Default for TrustDecayConfig {
    fn default() -> Self {
        Self {
            inactivity_penalty_per_30_days: 2.0,
            max_inactivity_penalty: 20.0,
        }
    }
}

impl Default for TrustTierThresholds {
    fn default() -> Self {
        Self {
            silver: 40.0,
            gold: 60.0,
            platinum: 75.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProfileCompletenessConfig {
    pub basic_info_points: f64,
    pub location_points: f64,
    pub business_details_points: f64,
    pub service_radius_points: f64,
    pub role_profile_points: f64,
}

impl Default for ProfileCompletenessConfig {
    fn default() -> Self {
        Self {
            basic_info_points: 20.0,
            location_points: 20.0,
            business_details_points: 20.0,
            service_radius_points: 10.0,
            role_profile_points: 30.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    pub party_id: Uuid,
    pub overall_score: f64,
    pub as_supplier_score: Option<f64>,
    pub as_consumer_score: Option<f64>,
    pub as_enhancer_score: Option<f64>,
    pub tier: TrustTier,
    pub components: ScoreComponents,
    pub calculation_formula: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreComponents {
    pub transaction_history: f64,
    pub review_ratings: f64,
    pub profile_completeness: f64,
    pub verification_level: f64,
    pub response_rate: f64,
    pub dispute_history: f64,
    pub longevity: f64,
    pub community: f64,
}

impl TrustScoreRow {
    pub fn new(party_id: Uuid) -> Self {
        Self {
            id: Uuid::now_v7(),
            party_id,
            overall_score: 0.0,
            as_supplier_score: None,
            as_consumer_score: None,
            as_enhancer_score: None,
            deals_completed_count: 0,
            deals_cancelled_count: 0,
            deals_disputed_count: 0,
            timeouts_count: 0,
            no_shows_count: 0,
            total_completed_value: 0.0,
            average_response_hours: None,
            profile_completeness: 0.0,
            verification_level: 0,
            longevity_days: 0,
            calculation_formula: serde_json::Value::Object(Default::default()),
            last_calculated_at: None,
            next_calculation_at: None,
        }
    }
}
