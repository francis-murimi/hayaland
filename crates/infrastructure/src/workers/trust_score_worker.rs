use application::trust_scores::RecalculateAllTrustScores;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{info, warn};

/// Run the nightly trust-score recalculation worker loop.
pub async fn run_trust_score_worker(
    recalc_all: Arc<RecalculateAllTrustScores>,
    interval_duration: Duration,
    batch_size: usize,
) {
    let mut ticker = interval(interval_duration);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;
        run_tick(&recalc_all, batch_size).await;
    }
}

async fn run_tick(recalc_all: &RecalculateAllTrustScores, batch_size: usize) {
    match recalc_all.execute(batch_size).await {
        Ok(result) => {
            info!(
                processed = result.processed,
                failed = result.failed,
                "trust_score_worker_tick_complete"
            );
        }
        Err(err) => {
            warn!(error = %err, "trust_score_worker_tick_failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use application::test_helpers::{FakePartyRepo, FakeTrustScoreRepo};
    use application::trust_scores::RecalculateTrustScore;
    use domain::entities::trust_score::TrustScoreConfig;
    use std::sync::Arc;

    #[tokio::test]
    async fn tick_recalculates_all_scores() {
        let trust_repo = Arc::new(FakeTrustScoreRepo::default());
        let party_repo = Arc::new(FakePartyRepo::default());
        let recalc_one = Arc::new(RecalculateTrustScore::new(
            trust_repo.clone(),
            party_repo.clone(),
            TrustScoreConfig::default(),
        ));
        let recalc_all = Arc::new(RecalculateAllTrustScores::new(
            trust_repo.clone(),
            recalc_one,
        ));

        run_tick(&recalc_all, 10).await;
    }
}
