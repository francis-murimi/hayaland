use application::analytics::RefreshDailyMetrics;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{info, warn};

/// Run the analytics daily-metrics refresh worker loop.
pub async fn run_analytics_worker(
    refresh_metrics: Arc<RefreshDailyMetrics>,
    interval_duration: Duration,
) {
    let mut ticker = interval(interval_duration);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;
        run_tick(&refresh_metrics).await;
    }
}

async fn run_tick(refresh_metrics: &RefreshDailyMetrics) {
    let today = OffsetDateTime::now_utc().date();
    match refresh_metrics.execute(today).await {
        Ok(()) => {
            info!(date = %today, "analytics_worker_tick_complete");
        }
        Err(err) => {
            warn!(error = %err, date = %today, "analytics_worker_tick_failed");
        }
    }
}
