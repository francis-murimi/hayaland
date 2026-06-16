use crate::errors::ApplicationError;
use crate::trust_scores::RecalculateTrustScore;
use domain::repositories::TrustScoreRepository;
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Debug, Clone, Default)]
pub struct RecalculateAllResult {
    pub processed: usize,
    pub failed: usize,
}

/// Recalculate trust scores for all parties in batches.
#[derive(Clone)]
pub struct RecalculateAllTrustScores {
    trust_repo: Arc<dyn TrustScoreRepository>,
    recalc: Arc<RecalculateTrustScore>,
}

impl RecalculateAllTrustScores {
    pub fn new(
        trust_repo: Arc<dyn TrustScoreRepository>,
        recalc: Arc<RecalculateTrustScore>,
    ) -> Self {
        Self { trust_repo, recalc }
    }

    #[instrument(skip(self))]
    pub async fn execute(
        &self,
        batch_size: usize,
    ) -> Result<RecalculateAllResult, ApplicationError> {
        let mut offset = 0i64;
        let mut processed = 0usize;
        let mut failed = 0usize;

        loop {
            let ids = self
                .trust_repo
                .list_party_ids(batch_size as i64, offset)
                .await?;
            if ids.is_empty() {
                break;
            }

            for party_id in ids {
                match self.recalc.execute(party_id).await {
                    Ok(_) => processed += 1,
                    Err(err) => {
                        failed += 1;
                        tracing::warn!(%party_id, error = %err, "trust_score_recalculation_failed");
                    }
                }
            }

            offset += batch_size as i64;
        }

        info!(
            processed,
            failed, "trust_score_nightly_recalculation_complete"
        );

        Ok(RecalculateAllResult { processed, failed })
    }
}
