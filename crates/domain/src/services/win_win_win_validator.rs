use crate::entities::{DealRole, ValueDistribution};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Per-party feedback produced by the validator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyFeedback {
    pub net_gain: Decimal,
    pub roi_percent: Decimal,
}

/// Severity bucket returned by the validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationStatus {
    Excellent,
    Good,
    Fair,
    Poor,
    Blocked,
}

/// A single validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    pub message: String,
    pub party_role: Option<DealRole>,
}

/// Full result of a Win-Win-Win validation run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationResult {
    pub score: Decimal,
    pub status: ValidationStatus,
    pub blocked: bool,
    pub violations: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
    pub party_feedback: BTreeMap<DealRole, PartyFeedback>,
}

/// Configuration for the validator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub min_deal_value: Decimal,
    pub max_share_percentage: Decimal,
    pub min_share_percentage: Decimal,
    pub max_risk_ratio: Decimal,
    pub enhancer_share_low_threshold: Decimal,
    pub enhancer_share_high_threshold: Decimal,
    pub max_upfront_percentage: Decimal,
    pub score_weights: ScoreWeights,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            min_deal_value: Decimal::from(500),
            max_share_percentage: Decimal::from(70),
            min_share_percentage: Decimal::from(5),
            max_risk_ratio: Decimal::from(3),
            enhancer_share_low_threshold: Decimal::from(10),
            enhancer_share_high_threshold: Decimal::from(40),
            max_upfront_percentage: Decimal::from(80),
            score_weights: ScoreWeights::default(),
        }
    }
}

/// Weighting of the four score components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreWeights {
    pub absolute_gain: Decimal,
    pub proportional_fairness: Decimal,
    pub market_benchmark: Decimal,
    pub opportunity_cost: Decimal,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            absolute_gain: Decimal::from(25),
            proportional_fairness: Decimal::from(30),
            market_benchmark: Decimal::from(25),
            opportunity_cost: Decimal::from(20),
        }
    }
}

/// Snapshot data needed per party.
#[derive(Debug, Clone, Default)]
pub struct PartyValidationSnapshot {
    pub trust_score: f64,
    pub active_deals: i64,
}

/// Input to the validator.
#[derive(Debug, Clone)]
pub struct ValidationInput {
    pub value_distribution: ValueDistribution,
    pub supplier: PartyValidationSnapshot,
    pub consumer: PartyValidationSnapshot,
    pub enhancer: PartyValidationSnapshot,
    pub all_mandatory_terms_accepted: bool,
    pub market_benchmark_premium: Option<Decimal>,
}

/// Pure domain service that validates a deal's fairness.
pub struct WinWinWinValidator;

impl WinWinWinValidator {
    pub fn validate(input: &ValidationInput, config: &ValidationConfig) -> ValidationResult {
        let mut violations = Vec::new();
        let mut warnings = Vec::new();
        let vd = &input.value_distribution;

        // Critical rules.
        if vd.total_value < config.min_deal_value {
            violations.push(ValidationIssue {
                code: "DEAL_VALUE_TOO_SMALL".to_string(),
                message: format!(
                    "total value {} is below minimum {}",
                    vd.total_value, config.min_deal_value
                ),
                party_role: None,
            });
        }

        if vd.supplier_share_percentage > config.max_share_percentage {
            violations.push(ValidationIssue {
                code: "SHARE_EXCEEDS_MAX".to_string(),
                message: format!(
                    "supplier share {}% exceeds maximum {}%",
                    vd.supplier_share_percentage, config.max_share_percentage
                ),
                party_role: Some(DealRole::Supplier),
            });
        }
        if vd.enhancer_share_percentage > config.max_share_percentage {
            violations.push(ValidationIssue {
                code: "SHARE_EXCEEDS_MAX".to_string(),
                message: format!(
                    "enhancer share {}% exceeds maximum {}%",
                    vd.enhancer_share_percentage, config.max_share_percentage
                ),
                party_role: Some(DealRole::Enhancer),
            });
        }
        if vd.platform_fee_percentage > config.max_share_percentage {
            violations.push(ValidationIssue {
                code: "PLATFORM_FEE_EXCEEDS_MAX".to_string(),
                message: format!(
                    "platform fee {}% exceeds maximum {}%",
                    vd.platform_fee_percentage, config.max_share_percentage
                ),
                party_role: None,
            });
        }

        if vd.supplier_share_amount <= Decimal::ZERO {
            violations.push(ValidationIssue {
                code: "SUPPLIER_GAIN_NOT_POSITIVE".to_string(),
                message: "supplier share must be greater than zero".to_string(),
                party_role: Some(DealRole::Supplier),
            });
        }
        if vd.enhancer_share_amount <= Decimal::ZERO {
            violations.push(ValidationIssue {
                code: "ENHANCER_GAIN_NOT_POSITIVE".to_string(),
                message: "enhancer share must be greater than zero".to_string(),
                party_role: Some(DealRole::Enhancer),
            });
        }
        if vd.consumer_cost_amount > vd.total_value {
            violations.push(ValidationIssue {
                code: "CONSUMER_OVERPAYS".to_string(),
                message: "consumer cost cannot exceed total deal value".to_string(),
                party_role: Some(DealRole::Consumer),
            });
        }

        if !input.all_mandatory_terms_accepted {
            violations.push(ValidationIssue {
                code: "MANDATORY_TERMS_NOT_ACCEPTED".to_string(),
                message: "all mandatory terms must be accepted".to_string(),
                party_role: None,
            });
        }

        // Warning rules.
        if vd.supplier_share_percentage < config.min_share_percentage {
            warnings.push(ValidationIssue {
                code: "SUPPLIER_SHARE_LOW".to_string(),
                message: format!(
                    "supplier share {}% is below {}% guidance",
                    vd.supplier_share_percentage, config.min_share_percentage
                ),
                party_role: Some(DealRole::Supplier),
            });
        }
        if vd.enhancer_share_percentage < config.enhancer_share_low_threshold {
            warnings.push(ValidationIssue {
                code: "ENHANCER_SHARE_LOW".to_string(),
                message: format!(
                    "enhancer share {}% is below {}% guidance",
                    vd.enhancer_share_percentage, config.enhancer_share_low_threshold
                ),
                party_role: Some(DealRole::Enhancer),
            });
        }
        if vd.enhancer_share_percentage > config.enhancer_share_high_threshold {
            warnings.push(ValidationIssue {
                code: "ENHANCER_SHARE_HIGH".to_string(),
                message: format!(
                    "enhancer share {}% is above {}% guidance",
                    vd.enhancer_share_percentage, config.enhancer_share_high_threshold
                ),
                party_role: Some(DealRole::Enhancer),
            });
        }

        // Risk ratio between largest and smallest party share.
        let party_shares = [vd.supplier_share_percentage, vd.enhancer_share_percentage];
        let max_share = party_shares.iter().cloned().max().unwrap_or(Decimal::ZERO);
        let min_share = party_shares
            .iter()
            .cloned()
            .filter(|s| *s > Decimal::ZERO)
            .min()
            .unwrap_or(Decimal::ONE);
        if max_share / min_share > config.max_risk_ratio {
            warnings.push(ValidationIssue {
                code: "RISK_RATIO_HIGH".to_string(),
                message: "risk ratio between largest and smallest party share is too high"
                    .to_string(),
                party_role: None,
            });
        }

        // Front-loaded payment schedule warning.
        let upfront_total: Decimal = vd
            .payment_schedule
            .iter()
            .filter(|e| matches!(e.trigger, crate::entities::PaymentTrigger::Upfront))
            .map(|e| e.amount)
            .sum();
        if !vd.total_value.is_zero()
            && (upfront_total / vd.total_value) * Decimal::from(100) > config.max_upfront_percentage
        {
            warnings.push(ValidationIssue {
                code: "PAYMENT_SCHEDULE_FRONT_LOADED".to_string(),
                message: "more than 80% of value is scheduled upfront".to_string(),
                party_role: None,
            });
        }

        // Party feedback.
        let consumer_net_gain = vd.total_value - vd.consumer_cost_amount;
        let mut party_feedback = BTreeMap::new();
        party_feedback.insert(
            DealRole::Supplier,
            PartyFeedback {
                net_gain: vd.supplier_share_amount,
                roi_percent: Self::safe_percent(vd.supplier_share_amount, vd.total_value),
            },
        );
        party_feedback.insert(
            DealRole::Consumer,
            PartyFeedback {
                net_gain: consumer_net_gain,
                roi_percent: Self::safe_percent(consumer_net_gain, vd.total_value),
            },
        );
        party_feedback.insert(
            DealRole::Enhancer,
            PartyFeedback {
                net_gain: vd.enhancer_share_amount,
                roi_percent: Self::safe_percent(vd.enhancer_share_amount, vd.total_value),
            },
        );

        // Score.
        let score = if violations.is_empty() {
            Self::compute_score(input, config)
        } else {
            Decimal::ZERO
        };

        let status = if !violations.is_empty() {
            ValidationStatus::Blocked
        } else if score >= Decimal::from(90) {
            ValidationStatus::Excellent
        } else if score >= Decimal::from(70) {
            ValidationStatus::Good
        } else if score >= Decimal::from(50) {
            ValidationStatus::Fair
        } else {
            ValidationStatus::Poor
        };

        ValidationResult {
            score,
            status,
            blocked: status == ValidationStatus::Blocked,
            violations,
            warnings,
            party_feedback,
        }
    }

    fn safe_percent(part: Decimal, whole: Decimal) -> Decimal {
        if whole.is_zero() {
            Decimal::ZERO
        } else {
            (part / whole) * Decimal::from(100)
        }
    }

    fn compute_score(input: &ValidationInput, config: &ValidationConfig) -> Decimal {
        let vd = &input.value_distribution;
        let weights = &config.score_weights;
        let total_weight = weights.absolute_gain
            + weights.proportional_fairness
            + weights.market_benchmark
            + weights.opportunity_cost;

        if total_weight.is_zero() {
            return Decimal::ZERO;
        }

        // Absolute gain: average normalized gain of the three parties.
        // The consumer's gain is the total value of the delivered output.
        let gains = [
            vd.supplier_share_amount,
            vd.enhancer_share_amount,
            vd.total_value,
        ];
        let avg_gain = gains.iter().copied().sum::<Decimal>() / Decimal::from(gains.len() as i32);
        let absolute_gain_score = Self::safe_percent(avg_gain, vd.total_value);

        // Proportional fairness: Gini coefficient of normalized gains.
        let normalized: Vec<Decimal> = gains
            .iter()
            .map(|g| Self::safe_percent(*g, vd.total_value))
            .collect();
        let mean =
            normalized.iter().copied().sum::<Decimal>() / Decimal::from(normalized.len() as i32);
        let fairness_score = if mean.is_zero() {
            Decimal::ZERO
        } else {
            let n = normalized.len() as i32;
            let mut absolute_diff_sum = Decimal::ZERO;
            for i in 0..normalized.len() {
                for j in 0..normalized.len() {
                    absolute_diff_sum += (normalized[i] - normalized[j]).abs();
                }
            }
            let gini = absolute_diff_sum / (Decimal::from(2) * Decimal::from(n * n) * mean);
            ((Decimal::from(1) - gini) * Decimal::from(100))
                .max(Decimal::ZERO)
                .min(Decimal::from(100))
        };

        // Market benchmark: optional premium; default 50 if absent.
        let market_benchmark_score = input
            .market_benchmark_premium
            .map(|p| (Decimal::from(50) + p).min(Decimal::from(100)))
            .unwrap_or_else(|| Decimal::from(50));

        // Opportunity cost: penalize parties with many active deals.
        let max_active = [
            input.supplier.active_deals,
            input.consumer.active_deals,
            input.enhancer.active_deals,
        ]
        .iter()
        .copied()
        .max()
        .unwrap_or(0);
        let opportunity_cost_score = if max_active > 3 {
            Decimal::from(50)
        } else {
            Decimal::from(100)
        };

        let weighted = absolute_gain_score * weights.absolute_gain
            + fairness_score * weights.proportional_fairness
            + market_benchmark_score * weights.market_benchmark
            + opportunity_cost_score * weights.opportunity_cost;

        (weighted / total_weight)
            .max(Decimal::ZERO)
            .min(Decimal::from(100))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{DistributionModel, ValueDistribution};
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn input_with_shares(supplier: i64, enhancer: i64, platform: i64) -> ValidationInput {
        let vd = ValueDistribution {
            id: Uuid::now_v7(),
            deal_id: Uuid::now_v7(),
            total_value: Decimal::from(10000),
            currency: "POINTS".to_string(),
            distribution_model: DistributionModel::FixedPrice,
            supplier_share_percentage: Decimal::from(supplier),
            supplier_share_amount: Decimal::from(10000 * supplier / 100),
            consumer_cost_percentage: Decimal::from(100),
            consumer_cost_amount: Decimal::from(10000),
            enhancer_share_percentage: Decimal::from(enhancer),
            enhancer_share_amount: Decimal::from(10000 * enhancer / 100),
            platform_fee_percentage: Decimal::from(platform),
            platform_fee_amount: Decimal::from(10000 * platform / 100),
            payment_schedule: vec![],
            win_win_win_score: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        };
        vd.validate().unwrap();

        ValidationInput {
            value_distribution: vd,
            supplier: PartyValidationSnapshot::default(),
            consumer: PartyValidationSnapshot::default(),
            enhancer: PartyValidationSnapshot::default(),
            all_mandatory_terms_accepted: true,
            market_benchmark_premium: None,
        }
    }

    #[test]
    fn good_deal_scores_good_or_better() {
        let input = input_with_shares(60, 30, 10);
        let result = WinWinWinValidator::validate(&input, &ValidationConfig::default());
        assert!(!result.blocked);
        assert!(result.score >= Decimal::from(70));
        assert!(matches!(
            result.status,
            ValidationStatus::Good | ValidationStatus::Excellent
        ));
    }

    #[test]
    fn unbalanced_share_is_blocked() {
        let input = input_with_shares(85, 5, 10);
        let result = WinWinWinValidator::validate(&input, &ValidationConfig::default());
        assert!(result.blocked);
        assert!(matches!(result.status, ValidationStatus::Blocked));
        assert!(result
            .violations
            .iter()
            .any(|v| v.code == "SHARE_EXCEEDS_MAX"));
    }

    #[test]
    fn low_enhancer_share_warns() {
        let input = input_with_shares(65, 5, 30);
        let result = WinWinWinValidator::validate(&input, &ValidationConfig::default());
        assert!(!result.blocked);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == "ENHANCER_SHARE_LOW"));
    }

    #[test]
    fn missing_mandatory_terms_blocks() {
        let mut input = input_with_shares(60, 30, 10);
        input.all_mandatory_terms_accepted = false;
        let result = WinWinWinValidator::validate(&input, &ValidationConfig::default());
        assert!(result.blocked);
    }
}
