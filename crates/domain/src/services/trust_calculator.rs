use crate::entities::trust_score::{
    ResponseMetrics, RoleDealInput, ScoreComponents, TrustColdStartConfig, TrustDecayConfig,
    TrustScore, TrustScoreConfig, TrustScoreInputs, TrustTier, TrustTierThresholds,
};
use uuid::Uuid;

const MIN_SCORE: f64 = 0.0;
const MAX_SCORE: f64 = 100.0;

pub struct TrustCalculator;

impl TrustCalculator {
    pub fn calculate(
        party_id: Uuid,
        inputs: &TrustScoreInputs,
        config: &TrustScoreConfig,
    ) -> TrustScore {
        let tx = transaction_history_score(inputs);
        let rev = review_ratings_score(inputs, &config.cold_start);
        let prof = profile_completeness_score(inputs);
        let ver = verification_level_score(inputs);
        let resp = response_rate_score(&inputs.response_metrics);
        let disp = dispute_history_score(inputs);
        let age = longevity_score(inputs);
        let comm = community_score(inputs);

        let components = ScoreComponents {
            transaction_history: clamp(tx),
            review_ratings: clamp(rev),
            profile_completeness: clamp(prof),
            verification_level: clamp(ver),
            response_rate: clamp(resp),
            dispute_history: clamp(disp),
            longevity: clamp(age),
            community: clamp(comm),
        };

        let weights = &config.weights;
        let raw = components.transaction_history * weights.transaction_history
            + components.review_ratings * weights.review_ratings
            + components.profile_completeness * weights.profile_completeness
            + components.verification_level * weights.verification_level
            + components.response_rate * weights.response_rate
            + components.dispute_history * weights.dispute_history
            + components.longevity * weights.longevity
            + components.community * weights.community;

        let with_decay = apply_decay(clamp(raw), inputs, &config.decay);
        let overall_score = clamp(with_decay);
        let tier = TrustTier::from_score(overall_score, &config.tiers);

        let calculation_formula = serde_json::json!({
            "overall": overall_score,
            "components": components,
            "weights": weights,
            "tier": tier,
            "inputs": {
                "deals_completed": inputs.deals_completed_count,
                "deals_cancelled": inputs.deals_cancelled_count,
                "deals_disputed": inputs.deals_disputed_count,
                "timeouts": inputs.timeouts_count,
                "no_shows": inputs.no_shows_count,
                "total_completed_value": inputs.total_completed_value,
                "review_count": inputs.reviews.len(),
                "dispute_count": inputs.disputes.len(),
                "response_hours": inputs.response_metrics.average_response_hours,
                "profile_completeness": inputs.profile_completeness,
                "verification_level": inputs.verification_level,
                "longevity_days": inputs.longevity_days,
                "days_since_last_activity": inputs.days_since_last_activity,
            }
        });

        TrustScore {
            party_id,
            overall_score,
            as_supplier_score: role_score("supplier", inputs, config),
            as_consumer_score: role_score("consumer", inputs, config),
            as_enhancer_score: role_score("enhancer", inputs, config),
            tier,
            components,
            calculation_formula,
        }
    }
}

fn clamp(v: f64) -> f64 {
    v.clamp(MIN_SCORE, MAX_SCORE)
}

fn transaction_history_score(inputs: &TrustScoreInputs) -> f64 {
    let completed = inputs.deals_completed_count.max(0) as f64;
    let cancelled = inputs.deals_cancelled_count.max(0) as f64;
    let total = completed + cancelled;
    if total == 0.0 {
        return 50.0;
    }
    let completion_rate = completed / total;
    let value_factor = (inputs.total_completed_value / 1000.0).atan() * 2.0 / std::f64::consts::PI;
    (completion_rate * 70.0 + value_factor * 30.0).min(100.0)
}

fn review_ratings_score(inputs: &TrustScoreInputs, cold_start: &TrustColdStartConfig) -> f64 {
    if inputs.reviews.is_empty() {
        return cold_start.global_average_review_score * 10.0; // 1-5 -> 10-50
    }

    let valid: Vec<_> = inputs
        .reviews
        .iter()
        .filter(|r| r.is_public && !r.is_hidden)
        .collect();
    if valid.is_empty() {
        return cold_start.global_average_review_score * 10.0;
    }

    let weighted: f64 = valid
        .iter()
        .map(|r| {
            let reviewer_weight = 0.5 + (r.reviewer_overall_score / 100.0) * 0.5;
            r.review_score * reviewer_weight
        })
        .sum();
    let own_avg = weighted / valid.len() as f64;
    let own_score = own_avg * 20.0; // 1-5 -> 20-100

    if valid.len() as i64 >= cold_start.min_reviews_before_own_score_dominates {
        own_score
    } else {
        let alpha = valid.len() as f64 / cold_start.min_reviews_before_own_score_dominates as f64;
        alpha * own_score + (1.0 - alpha) * cold_start.global_average_review_score * 10.0
    }
}

fn profile_completeness_score(inputs: &TrustScoreInputs) -> f64 {
    inputs.profile_completeness
}

fn verification_level_score(inputs: &TrustScoreInputs) -> f64 {
    match inputs.verification_level {
        0 => 0.0,
        1 => 25.0,
        2 => 50.0,
        3 => 75.0,
        _ => 100.0,
    }
}

fn response_rate_score(metrics: &ResponseMetrics) -> f64 {
    if metrics.messages_received_90d == 0 {
        return 50.0;
    }
    let rate = metrics.messages_responded_90d as f64 / metrics.messages_received_90d as f64;
    let speed_score = metrics.average_response_hours.map_or(50.0, |h| {
        if h <= 1.0 {
            100.0
        } else if h <= 24.0 {
            100.0 - (h - 1.0) / 23.0 * 30.0
        } else {
            70.0 - (h - 24.0).min(168.0) / 168.0 * 70.0
        }
    });
    (rate * 50.0 + speed_score * 50.0) / 100.0
}

fn dispute_history_score(inputs: &TrustScoreInputs) -> f64 {
    if inputs.disputes.is_empty() {
        return 100.0;
    }
    let total = inputs.disputes.len() as f64;
    let lost: f64 = inputs
        .disputes
        .iter()
        .filter(|d| {
            d.resolution_outcome
                .as_ref()
                .map(|o| o.eq_ignore_ascii_case("lost") || o.eq_ignore_ascii_case("partially_lost"))
                .unwrap_or(false)
        })
        .count() as f64;
    let lost_rate = lost / total;
    (1.0 - lost_rate) * 100.0
}

fn longevity_score(inputs: &TrustScoreInputs) -> f64 {
    let days = inputs.longevity_days.max(0) as f64;
    (days / 365.0 * 20.0).min(100.0)
}

fn community_score(inputs: &TrustScoreInputs) -> f64 {
    let activity_bonus = match inputs.days_since_last_activity {
        None => 0.0,
        Some(d) if d <= 7 => 20.0,
        Some(d) if d <= 30 => 10.0,
        Some(_) => 0.0,
    };
    let verif_bonus = match inputs.verification_level {
        0 => 0.0,
        1 => 5.0,
        2 => 10.0,
        3 => 15.0,
        _ => 20.0,
    };
    (50_f64 + activity_bonus + verif_bonus).min(100.0)
}

fn apply_decay(score: f64, inputs: &TrustScoreInputs, decay: &TrustDecayConfig) -> f64 {
    let Some(days) = inputs.days_since_last_activity else {
        return score;
    };
    if days <= 30 {
        return score;
    }
    let periods = (days as f64 / 30.0).floor();
    let penalty =
        (periods * decay.inactivity_penalty_per_30_days).min(decay.max_inactivity_penalty);
    score - penalty
}

fn role_score(role: &str, inputs: &TrustScoreInputs, config: &TrustScoreConfig) -> Option<f64> {
    let role_deal = inputs
        .role_deals
        .get(role)
        .cloned()
        .unwrap_or(RoleDealInput {
            deals_completed_count: 0,
            deals_cancelled_count: 0,
            total_completed_value: 0.0,
        });

    if role_deal.deals_completed_count == 0 && role_deal.deals_cancelled_count == 0 {
        return None;
    }

    let role_inputs = TrustScoreInputs {
        deals_completed_count: role_deal.deals_completed_count,
        deals_cancelled_count: role_deal.deals_cancelled_count,
        total_completed_value: role_deal.total_completed_value,
        reviews: inputs.role_reviews.get(role).cloned().unwrap_or_default(),
        ..clone_inputs(inputs)
    };

    let tx = transaction_history_score(&role_inputs);
    let rev = review_ratings_score(&role_inputs, &config.cold_start);
    let resp = response_rate_score(&inputs.response_metrics);
    let disp = dispute_history_score(inputs);

    let weights = &config.weights;
    let raw = tx * weights.transaction_history
        + rev * weights.review_ratings
        + resp * weights.response_rate
        + disp * weights.dispute_history
        + inputs.profile_completeness * 100.0 * weights.profile_completeness
        + verification_level_score(inputs) * weights.verification_level
        + longevity_score(inputs) * weights.longevity
        + community_score(inputs) * weights.community;

    Some(clamp(apply_decay(raw, inputs, &config.decay)))
}

fn clone_inputs(inputs: &TrustScoreInputs) -> TrustScoreInputs {
    TrustScoreInputs {
        deals_completed_count: inputs.deals_completed_count,
        deals_cancelled_count: inputs.deals_cancelled_count,
        deals_disputed_count: inputs.deals_disputed_count,
        timeouts_count: inputs.timeouts_count,
        no_shows_count: inputs.no_shows_count,
        total_completed_value: inputs.total_completed_value,
        reviews: inputs.reviews.clone(),
        disputes: inputs.disputes.clone(),
        role_deals: inputs.role_deals.clone(),
        role_reviews: inputs.role_reviews.clone(),
        response_metrics: inputs.response_metrics,
        profile_completeness: inputs.profile_completeness,
        verification_level: inputs.verification_level,
        longevity_days: inputs.longevity_days,
        days_since_last_activity: inputs.days_since_last_activity,
    }
}

pub fn tier_from_score(score: f64, thresholds: &TrustTierThresholds) -> TrustTier {
    TrustTier::from_score(score, thresholds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{DisputeInput, ReviewInput};
    use std::collections::HashMap;
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn default_config() -> TrustScoreConfig {
        TrustScoreConfig::default()
    }

    fn base_inputs() -> TrustScoreInputs {
        TrustScoreInputs {
            deals_completed_count: 0,
            deals_cancelled_count: 0,
            deals_disputed_count: 0,
            timeouts_count: 0,
            no_shows_count: 0,
            total_completed_value: 0.0,
            reviews: Vec::new(),
            disputes: Vec::new(),
            role_deals: HashMap::new(),
            role_reviews: HashMap::new(),
            response_metrics: ResponseMetrics {
                average_response_hours: None,
                messages_received_90d: 0,
                messages_responded_90d: 0,
            },
            profile_completeness: 0.0,
            verification_level: 0,
            longevity_days: 0,
            days_since_last_activity: None,
        }
    }

    #[test]
    fn calculate_with_empty_inputs_uses_defaults_and_cold_start() {
        let config = default_config();
        let inputs = base_inputs();
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        assert_eq!(score.components.transaction_history, 50.0);
        assert_eq!(score.components.review_ratings, 25.0);
        assert_eq!(score.components.response_rate, 50.0);
        assert_eq!(score.components.dispute_history, 100.0);
        assert_eq!(score.components.longevity, 0.0);
        assert_eq!(score.components.community, 50.0);
        assert_eq!(score.tier, TrustTier::Bronze);
        assert!(score.as_supplier_score.is_none());
        assert!(score.as_consumer_score.is_none());
        assert!(score.as_enhancer_score.is_none());
    }

    #[test]
    fn transaction_history_score_combines_completion_rate_and_value() {
        let config = default_config();
        let mut inputs = base_inputs();
        inputs.deals_completed_count = 8;
        inputs.deals_cancelled_count = 2;
        inputs.total_completed_value = 1000.0;
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        assert_eq!(score.components.transaction_history, 71.0);
    }

    #[test]
    fn review_ratings_uses_cold_start_when_no_reviews() {
        let config = default_config();
        let inputs = base_inputs();
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        assert_eq!(score.components.review_ratings, 25.0);
    }

    #[test]
    fn review_ratings_ignores_hidden_reviews() {
        let config = default_config();
        let mut inputs = base_inputs();
        inputs.reviews.push(ReviewInput {
            reviewer_party_id: None,
            reviewer_overall_score: 80.0,
            review_score: 5.0,
            deal_value: 100.0,
            created_at: OffsetDateTime::now_utc(),
            is_public: false,
            is_hidden: true,
        });
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        assert_eq!(score.components.review_ratings, 25.0);
    }

    #[test]
    fn review_ratings_blends_with_cold_start_below_threshold() {
        let config = default_config();
        let mut inputs = base_inputs();
        inputs.reviews.push(ReviewInput {
            reviewer_party_id: None,
            reviewer_overall_score: 100.0,
            review_score: 5.0,
            deal_value: 100.0,
            created_at: OffsetDateTime::now_utc(),
            is_public: true,
            is_hidden: false,
        });
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        // One review out of 3 threshold -> alpha = 1/3
        // own_score = 5.0 * (0.5 + 1.0 * 0.5) * 20 = 100
        // cold = 2.5 * 10 = 25
        assert!((score.components.review_ratings - 50.0).abs() < 0.001);
    }

    #[test]
    fn review_ratings_uses_own_score_when_enough_reviews() {
        let config = default_config();
        let mut inputs = base_inputs();
        for _ in 0..3 {
            inputs.reviews.push(ReviewInput {
                reviewer_party_id: None,
                reviewer_overall_score: 100.0,
                review_score: 4.0,
                deal_value: 100.0,
                created_at: OffsetDateTime::now_utc(),
                is_public: true,
                is_hidden: false,
            });
        }
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        // weighted avg = 4.0 * 1.0 = 4.0, own_score = 80.0
        assert_eq!(score.components.review_ratings, 80.0);
    }

    #[test]
    fn verification_level_score_steps() {
        let config = default_config();
        for (level, expected) in [(0, 0.0), (1, 25.0), (2, 50.0), (3, 75.0), (5, 100.0)] {
            let mut inputs = base_inputs();
            inputs.verification_level = level;
            let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);
            assert_eq!(
                score.components.verification_level, expected,
                "level {level}"
            );
        }
    }

    #[test]
    fn response_rate_score_with_no_messages_defaults_to_fifty() {
        let config = default_config();
        let inputs = base_inputs();
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        assert_eq!(score.components.response_rate, 50.0);
    }

    #[test]
    fn response_rate_score_combines_rate_and_speed() {
        let config = default_config();
        let mut inputs = base_inputs();
        inputs.response_metrics = ResponseMetrics {
            average_response_hours: Some(12.0),
            messages_received_90d: 10,
            messages_responded_90d: 10,
        };
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        // Existing formula: (rate * 50 + speed * 50) / 100
        // rate = 1.0, speed = 100 - (11/23)*30 ≈ 85.65
        let speed = 100.0 - 11.0 / 23.0 * 30.0;
        let expected = (1.0 * 50.0 + speed * 50.0) / 100.0;
        assert!((score.components.response_rate - expected).abs() < 0.001);
    }

    #[test]
    fn response_rate_speed_branches() {
        let config = default_config();
        let cases = [
            (0.5, 100.0),
            (1.0, 100.0),
            (24.0, 70.0),
            (48.0, 70.0 - 24.0 / 168.0 * 70.0),
            (200.0, 0.0),
        ];
        for (hours, speed) in cases {
            let mut inputs = base_inputs();
            inputs.response_metrics = ResponseMetrics {
                average_response_hours: Some(hours),
                messages_received_90d: 10,
                messages_responded_90d: 10,
            };
            let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);
            // With rate = 1.0 the existing formula yields (50 + speed * 50) / 100.
            let expected = (50.0 + speed * 50.0) / 100.0;
            assert!(
                (score.components.response_rate - expected).abs() < 0.001,
                "hours {hours}"
            );
        }
    }

    #[test]
    fn dispute_history_score_punishes_lost_disputes() {
        let config = default_config();
        let mut inputs = base_inputs();
        inputs.disputes.push(DisputeInput {
            raised_by_party_id: Uuid::now_v7(),
            against_party_id: None,
            resolution_type: Some("refund".into()),
            resolution_outcome: Some("lost".into()),
            created_at: OffsetDateTime::now_utc(),
            resolved_at: None,
        });
        inputs.disputes.push(DisputeInput {
            raised_by_party_id: Uuid::now_v7(),
            against_party_id: None,
            resolution_type: Some("refund".into()),
            resolution_outcome: Some("won".into()),
            created_at: OffsetDateTime::now_utc(),
            resolved_at: None,
        });
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        assert_eq!(score.components.dispute_history, 50.0);
    }

    #[test]
    fn partially_lost_dispute_counts_as_lost() {
        let config = default_config();
        let mut inputs = base_inputs();
        inputs.disputes.push(DisputeInput {
            raised_by_party_id: Uuid::now_v7(),
            against_party_id: None,
            resolution_type: None,
            resolution_outcome: Some("partially_lost".into()),
            created_at: OffsetDateTime::now_utc(),
            resolved_at: None,
        });
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        assert_eq!(score.components.dispute_history, 0.0);
    }

    #[test]
    fn longevity_score_caps_at_100() {
        let config = default_config();
        let mut inputs = base_inputs();
        inputs.longevity_days = 365 * 10;
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        assert_eq!(score.components.longevity, 100.0);
    }

    #[test]
    fn community_score_activity_and_verification_bonuses() {
        let config = default_config();
        let mut inputs = base_inputs();
        inputs.days_since_last_activity = Some(3);
        inputs.verification_level = 3;
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        assert_eq!(score.components.community, 85.0);
    }

    #[test]
    fn apply_decay_reduces_score_after_30_days() {
        let config = default_config();
        let mut inputs = base_inputs();
        inputs.days_since_last_activity = Some(65);
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        // 2 full 30-day periods -> penalty 4.0
        assert!(score.overall_score < 50.0);
    }

    #[test]
    fn role_scores_computed_when_role_deals_present() {
        let config = default_config();
        let mut inputs = base_inputs();
        inputs.role_deals.insert(
            "supplier".into(),
            RoleDealInput {
                deals_completed_count: 10,
                deals_cancelled_count: 0,
                total_completed_value: 500.0,
            },
        );
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        assert!(score.as_supplier_score.is_some());
        assert!(score.as_supplier_score.unwrap() > 0.0);
        assert!(score.as_consumer_score.is_none());
        assert!(score.as_enhancer_score.is_none());
    }

    #[test]
    fn role_score_with_reviews() {
        let config = default_config();
        let mut inputs = base_inputs();
        inputs.role_deals.insert(
            "consumer".into(),
            RoleDealInput {
                deals_completed_count: 5,
                deals_cancelled_count: 0,
                total_completed_value: 100.0,
            },
        );
        inputs.role_reviews.insert(
            "consumer".into(),
            vec![ReviewInput {
                reviewer_party_id: None,
                reviewer_overall_score: 80.0,
                review_score: 5.0,
                deal_value: 100.0,
                created_at: OffsetDateTime::now_utc(),
                is_public: true,
                is_hidden: false,
            }],
        );
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        assert!(score.as_consumer_score.is_some());
    }

    #[test]
    fn tier_changes_based_on_score() {
        let thresholds = TrustTierThresholds::default();
        assert_eq!(tier_from_score(30.0, &thresholds), TrustTier::Bronze);
        assert_eq!(tier_from_score(45.0, &thresholds), TrustTier::Silver);
        assert_eq!(tier_from_score(65.0, &thresholds), TrustTier::Gold);
        assert_eq!(tier_from_score(80.0, &thresholds), TrustTier::Platinum);
    }

    #[test]
    fn scores_are_clamped_between_zero_and_one_hundred() {
        let config = default_config();
        let mut inputs = base_inputs();
        inputs.profile_completeness = 150.0;
        inputs.verification_level = 10;
        let score = TrustCalculator::calculate(Uuid::now_v7(), &inputs, &config);

        assert_eq!(score.components.profile_completeness, 100.0);
        assert_eq!(score.components.verification_level, 100.0);
    }
}
