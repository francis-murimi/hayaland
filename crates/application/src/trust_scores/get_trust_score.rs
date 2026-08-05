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

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use crate::test_helpers::{FakePartyRepo, FakeTrustScoreRepo};
    use domain::entities::trust_score::TrustScoreRow;
    use domain::entities::{DisplayName, Email, Party, PartyType};
    use uuid::Uuid;

    fn party(id: Uuid) -> Party {
        Party::new(
            id,
            PartyType::Organization,
            DisplayName::new("Acme Corp").unwrap(),
            Email::new("acme@example.com").unwrap(),
        )
    }

    #[tokio::test]
    async fn get_returns_existing_row_with_formula_breakdown() {
        let repo = Arc::new(FakeTrustScoreRepo::default());
        let party_id = Uuid::now_v7();
        let mut row = TrustScoreRow::new(party_id);
        row.overall_score = 73.0;
        row.deals_completed_count = 3;
        row.deals_cancelled_count = 1;
        row.calculation_formula = serde_json::json!({
            "tier": "GOLD",
            "inputs": {"review_count": 4},
            "components": {
                "transaction_history": 80.0,
                "review_ratings": 90.0,
                "non_numeric": "skip"
            }
        });
        repo.upsert(&row).await.unwrap();

        let uc = GetTrustScore::new(repo);
        let result = uc.execute(party_id).await.unwrap();
        assert_eq!(result.trust_score_id, row.id);
        assert_eq!(result.party_id, party_id);
        assert_eq!(result.overall_score, 73.0);
        assert!((result.score_out_of_5 - 3.65).abs() < 0.001);
        assert_eq!(result.tier, "GOLD");
        assert_eq!(result.detailed_metrics.total_reviews, 4);
        assert_eq!(result.detailed_metrics.completion_rate, 0.75);
        assert_eq!(result.detailed_metrics.average_rating, Some(4.5));
        assert_eq!(result.component_breakdown.len(), 2);
        assert!(result.role_scores.as_supplier.is_none());
    }

    #[tokio::test]
    async fn get_creates_default_when_missing() {
        let repo = Arc::new(FakeTrustScoreRepo::default());
        let party_id = Uuid::now_v7();
        let uc = GetTrustScore::new(repo.clone());
        let result = uc.execute(party_id).await.unwrap();
        assert_eq!(result.party_id, party_id);
        assert_eq!(result.tier, "BRONZE");
        assert_eq!(result.detailed_metrics.completion_rate, 0.0);
        assert!(repo.find_by_party_id(party_id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn get_uses_role_scores_and_response_metrics_when_present() {
        let repo = Arc::new(FakeTrustScoreRepo::default());
        let party_id = Uuid::now_v7();
        let mut row = TrustScoreRow::new(party_id);
        row.as_supplier_score = Some(60.0);
        row.as_consumer_score = Some(70.0);
        row.as_enhancer_score = Some(80.0);
        row.average_response_hours = Some(2.5);
        repo.upsert(&row).await.unwrap();

        let uc = GetTrustScore::new(repo);
        let result = uc.execute(party_id).await.unwrap();
        assert_eq!(result.role_scores.as_supplier.unwrap().score, 60.0);
        assert_eq!(result.role_scores.as_consumer.unwrap().score, 70.0);
        assert_eq!(result.role_scores.as_enhancer.unwrap().score, 80.0);
        assert_eq!(result.detailed_metrics.average_response_hours, Some(2.5));
    }
}
