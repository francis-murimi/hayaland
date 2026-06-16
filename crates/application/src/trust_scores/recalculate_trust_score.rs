use crate::errors::ApplicationError;
use crate::trust_scores::dto::TrustScoreResult;
use crate::trust_scores::profile_completeness::ProfileCompletenessCalculator;
use domain::entities::trust_score::{
    TrustScore, TrustScoreConfig, TrustScoreInputs, TrustScoreRow,
};
use domain::repositories::{PartyRepository, TrustScoreRepository};
use domain::services::TrustCalculator;

use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{info, instrument};
use uuid::Uuid;

/// Recalculate and persist a party's trust score.
#[derive(Clone)]
pub struct RecalculateTrustScore {
    trust_repo: Arc<dyn TrustScoreRepository>,
    party_repo: Arc<dyn PartyRepository>,
    config: TrustScoreConfig,
}

impl RecalculateTrustScore {
    pub fn new(
        trust_repo: Arc<dyn TrustScoreRepository>,
        party_repo: Arc<dyn PartyRepository>,
        config: TrustScoreConfig,
    ) -> Self {
        Self {
            trust_repo,
            party_repo,
            config,
        }
    }

    #[instrument(skip(self), fields(party_id = %party_id))]
    pub async fn execute(&self, party_id: Uuid) -> Result<TrustScoreResult, ApplicationError> {
        let existing = self.trust_repo.find_by_party_id(party_id).await?;
        if existing.is_none() {
            self.trust_repo.create_default(party_id).await?;
        }

        let party = self
            .party_repo
            .find_by_id(party_id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;

        let inputs = self.collect_inputs(party_id).await?;
        let trust_score = TrustCalculator::calculate(party_id, &inputs, &self.config);

        let row = TrustScoreRow {
            id: existing.map(|r| r.id).unwrap_or_else(Uuid::now_v7),
            party_id,
            overall_score: trust_score.overall_score,
            as_supplier_score: trust_score.as_supplier_score,
            as_consumer_score: trust_score.as_consumer_score,
            as_enhancer_score: trust_score.as_enhancer_score,
            deals_completed_count: inputs.deals_completed_count,
            deals_cancelled_count: inputs.deals_cancelled_count,
            deals_disputed_count: inputs.deals_disputed_count,
            timeouts_count: inputs.timeouts_count,
            no_shows_count: inputs.no_shows_count,
            total_completed_value: inputs.total_completed_value,
            average_response_hours: inputs.response_metrics.average_response_hours,
            profile_completeness: inputs.profile_completeness,
            verification_level: inputs.verification_level,
            longevity_days: inputs.longevity_days,
            calculation_formula: trust_score.calculation_formula.clone(),
            last_calculated_at: Some(OffsetDateTime::now_utc()),
            next_calculation_at: Some(OffsetDateTime::now_utc() + time::Duration::hours(24)),
        };

        self.trust_repo.upsert(&row).await?;
        self.trust_repo
            .update_public_cache(party_id, trust_score.overall_score)
            .await?;

        info!(%party_id, overall_score = %trust_score.overall_score, "trust score recalculated");

        Ok(map_to_result(
            row,
            trust_score,
            party.display_name.as_str().to_owned(),
        ))
    }

    async fn collect_inputs(&self, party_id: Uuid) -> Result<TrustScoreInputs, ApplicationError> {
        let trust_row = self
            .trust_repo
            .find_by_party_id(party_id)
            .await?
            .unwrap_or_else(|| TrustScoreRow::new(party_id));

        let reviews = self.trust_repo.find_review_inputs(party_id).await?;
        let disputes = self.trust_repo.find_dispute_inputs(party_id).await?;
        let role_deals = self.trust_repo.find_role_deal_inputs(party_id).await?;
        let role_reviews = self.trust_repo.find_role_reviews(party_id).await?;
        let response_metrics = self.trust_repo.compute_response_metrics(party_id).await?;
        let (longevity_days, days_since_last_activity) = self
            .trust_repo
            .find_account_age_and_activity(party_id)
            .await?;

        let party = self
            .party_repo
            .find_by_id(party_id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;
        let role_pairs = self.party_repo.list_roles(party_id).await?;
        let profile_completeness =
            ProfileCompletenessCalculator::calculate(&party, &role_pairs, &self.config);

        Ok(TrustScoreInputs {
            deals_completed_count: trust_row.deals_completed_count,
            deals_cancelled_count: trust_row.deals_cancelled_count,
            deals_disputed_count: trust_row.deals_disputed_count,
            timeouts_count: trust_row.timeouts_count,
            no_shows_count: trust_row.no_shows_count,
            total_completed_value: trust_row.total_completed_value,
            reviews,
            profile_completeness,
            verification_level: trust_row.verification_level,
            response_metrics,
            disputes,
            longevity_days,
            days_since_last_activity,
            role_deals,
            role_reviews,
        })
    }
}

fn map_to_result(row: TrustScoreRow, score: TrustScore, display_name: String) -> TrustScoreResult {
    let total_deals = row.deals_completed_count + row.deals_cancelled_count;
    let completion_rate = if total_deals > 0 {
        row.deals_completed_count as f64 / total_deals as f64
    } else {
        0.0
    };

    let component_breakdown: std::collections::HashMap<String, f64> =
        serde_json::from_value(score.calculation_formula.clone())
            .ok()
            .and_then(|v: serde_json::Value| v.get("components").cloned())
            .and_then(|c| serde_json::from_value(c).ok())
            .unwrap_or_else(|| {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    "transaction_history".to_string(),
                    score.components.transaction_history,
                );
                m.insert(
                    "review_ratings".to_string(),
                    score.components.review_ratings,
                );
                m.insert(
                    "profile_completeness".to_string(),
                    score.components.profile_completeness,
                );
                m.insert(
                    "verification_level".to_string(),
                    score.components.verification_level,
                );
                m.insert("response_rate".to_string(), score.components.response_rate);
                m.insert(
                    "dispute_history".to_string(),
                    score.components.dispute_history,
                );
                m.insert("longevity".to_string(), score.components.longevity);
                m.insert("community".to_string(), score.components.community);
                m
            });

    let total_reviews = row
        .calculation_formula
        .get("inputs")
        .and_then(|i| i.get("review_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    TrustScoreResult {
        trust_score_id: row.id,
        party_id: row.party_id,
        party_name: display_name,
        overall_score: score.overall_score,
        score_out_of_5: (score.overall_score / 20.0).clamp(0.0, 5.0),
        tier: format!("{:?}", score.tier).to_uppercase(),
        role_scores: crate::trust_scores::dto::RoleScoresResult {
            as_supplier: score
                .as_supplier_score
                .map(|s| crate::trust_scores::dto::RoleScoreResult { score: s }),
            as_consumer: score
                .as_consumer_score
                .map(|s| crate::trust_scores::dto::RoleScoreResult { score: s }),
            as_enhancer: score
                .as_enhancer_score
                .map(|s| crate::trust_scores::dto::RoleScoreResult { score: s }),
        },
        detailed_metrics: crate::trust_scores::dto::DetailedMetrics {
            deals_completed_count: row.deals_completed_count,
            deals_cancelled_count: row.deals_cancelled_count,
            deals_disputed_count: row.deals_disputed_count,
            timeouts_count: row.timeouts_count,
            no_shows_count: row.no_shows_count,
            completion_rate,
            average_response_hours: row.average_response_hours,
            profile_completeness: row.profile_completeness,
            verification_level: row.verification_level,
            longevity_days: row.longevity_days,
            total_reviews,
            average_rating: component_breakdown.get("review_ratings").map(|s| s / 20.0),
        },
        component_breakdown,
        last_calculated_at: row.last_calculated_at,
        next_calculation_at: row.next_calculation_at,
        calculation_formula: row.calculation_formula,
    }
}
