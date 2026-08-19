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

#[cfg(test)]
mod tests {
    use super::*;
    use application::analytics::RefreshDailyMetrics;
    use async_trait::async_trait;
    use domain::errors::DomainError;
    use domain::repositories::{
        AnalyticsRepository, DashboardSummary, DealTrend, MetricFilters, MetricsListResult,
        PartyActivityMetric,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use time::Date;

    struct CountingAnalyticsRepo {
        refresh_count: AtomicUsize,
        fail: bool,
    }

    impl CountingAnalyticsRepo {
        fn new() -> Self {
            Self {
                refresh_count: AtomicUsize::new(0),
                fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                refresh_count: AtomicUsize::new(0),
                fail: true,
            }
        }
    }

    #[async_trait]
    impl AnalyticsRepository for CountingAnalyticsRepo {
        async fn refresh_daily_metrics(&self, _date: Date) -> Result<(), DomainError> {
            if self.fail {
                return Err(DomainError::RepositoryError("boom".to_string()));
            }
            self.refresh_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn get_dashboard_summary(&self) -> Result<DashboardSummary, DomainError> {
            Ok(DashboardSummary::default())
        }

        async fn get_deal_trends(
            &self,
            _from: Date,
            _to: Date,
        ) -> Result<Vec<DealTrend>, DomainError> {
            Ok(vec![])
        }

        async fn get_party_activity(
            &self,
            _from: Date,
            _to: Date,
        ) -> Result<Vec<PartyActivityMetric>, DomainError> {
            Ok(vec![])
        }

        async fn list_daily_metrics(
            &self,
            _filters: MetricFilters,
        ) -> Result<MetricsListResult, DomainError> {
            Ok(MetricsListResult {
                items: vec![],
                total: 0,
            })
        }
    }

    #[tokio::test]
    async fn worker_runs_refresh_on_tick() {
        let repo = Arc::new(CountingAnalyticsRepo::new());
        let refresh = Arc::new(RefreshDailyMetrics::new(repo.clone()));

        let handle = tokio::spawn(run_analytics_worker(refresh, Duration::from_millis(10)));

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;

        assert!(
            repo.refresh_count.load(Ordering::SeqCst) >= 1,
            "worker should have refreshed at least once"
        );
    }

    #[tokio::test]
    async fn worker_continues_after_failed_tick() {
        let repo = Arc::new(CountingAnalyticsRepo::failing());
        let refresh = Arc::new(RefreshDailyMetrics::new(repo.clone()));

        let handle = tokio::spawn(run_analytics_worker(refresh, Duration::from_millis(10)));

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;

        assert_eq!(repo.refresh_count.load(Ordering::SeqCst), 0);
    }
}
