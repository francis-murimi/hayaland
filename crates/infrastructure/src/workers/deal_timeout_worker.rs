use application::deals::{ProcessDealTimeouts, ProcessDealTimeoutsResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{info, warn};

/// Run the deal timeout worker loop.
pub async fn run_deal_timeout_worker(
    process_timeouts: Arc<ProcessDealTimeouts>,
    interval_duration: Duration,
    batch_size: usize,
) {
    let mut ticker = interval(interval_duration);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;

        match process_timeouts.execute(batch_size).await {
            Ok(ProcessDealTimeoutsResult {
                transitioned,
                blocked,
                skipped,
                errors,
            }) => {
                let candidates = transitioned.len() + blocked.len() + skipped.len() + errors.len();
                info!(
                    candidates = candidates,
                    transitioned = transitioned.len(),
                    blocked = blocked.len(),
                    skipped = skipped.len(),
                    errors = errors.len(),
                    "deal_timeout_worker_tick_complete"
                );
            }
            Err(err) => {
                warn!(error = %err, "deal_timeout_worker_tick_failed");
            }
        }
    }
}
