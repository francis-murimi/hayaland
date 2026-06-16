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
}
