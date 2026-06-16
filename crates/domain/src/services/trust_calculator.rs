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
