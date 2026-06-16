use crate::errors::ApplicationError;
use crate::trust_scores::dto::TrustScoreResult;
use domain::repositories::TrustScoreRepository;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

/// Read a party's trust score.
#[derive(Clone)]
pub struct GetTrustScore {
    repo: Arc<dyn TrustScoreRepository>,
}

impl GetTrustScore {
    pub fn new(repo: Arc<dyn TrustScoreRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self), fields(party_id = %party_id))]
    pub async fn execute(&self, party_id: Uuid) -> Result<TrustScoreResult, ApplicationError> {
        let row = match self.repo.find_by_party_id(party_id).await? {
            Some(row) => row,
            None => {
                self.repo.create_default(party_id).await?;
                domain::entities::trust_score::TrustScoreRow::new(party_id)
            }
        };

        let completion_rate = {
            let total = row.deals_completed_count + row.deals_cancelled_count;
            if total > 0 {
                row.deals_completed_count as f64 / total as f64
            } else {
                0.0
            }
        };

        let total_reviews = row
            .calculation_formula
            .get("inputs")
            .and_then(|i| i.get("review_count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let component_breakdown: std::collections::HashMap<String, f64> = row
            .calculation_formula
            .get("components")
            .and_then(|c| c.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_f64().map(|score| (k.clone(), score)))
                    .collect()
            })
            .unwrap_or_default();

        let tier = row
            .calculation_formula
            .get("tier")
            .and_then(|t| t.as_str())
            .unwrap_or("BRONZE")
            .to_string();

        Ok(TrustScoreResult {
            trust_score_id: row.id,
            party_id: row.party_id,
            party_name: String::new(),
            overall_score: row.overall_score,
            score_out_of_5: (row.overall_score / 20.0).clamp(0.0, 5.0),
            tier,
            role_scores: super::dto::RoleScoresResult {
                as_supplier: row
                    .as_supplier_score
                    .map(|s| super::dto::RoleScoreResult { score: s }),
                as_consumer: row
                    .as_consumer_score
                    .map(|s| super::dto::RoleScoreResult { score: s }),
                as_enhancer: row
                    .as_enhancer_score
                    .map(|s| super::dto::RoleScoreResult { score: s }),
            },
            detailed_metrics: super::dto::DetailedMetrics {
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
        })
    }
}
