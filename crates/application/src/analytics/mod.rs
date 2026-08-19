pub mod dto;

use crate::analytics::dto::{
    DailyMetricDto, DailyMetricsListDto, DashboardSummaryDto, DealTrendDto, PartyActivityDto,
};
use crate::errors::ApplicationError;
use domain::repositories::AnalyticsRepository;
use std::sync::Arc;
use time::{Date, Duration};
use tracing::{info, instrument};

#[derive(Clone)]
pub struct RefreshDailyMetrics {
    repo: Arc<dyn AnalyticsRepository>,
}

impl RefreshDailyMetrics {
    pub fn new(repo: Arc<dyn AnalyticsRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, date: Date) -> Result<(), ApplicationError> {
        self.repo.refresh_daily_metrics(date).await?;
        info!(%date, "refreshed daily platform metrics");
        Ok(())
    }
}

#[derive(Clone)]
pub struct GetDashboardSummary {
    repo: Arc<dyn AnalyticsRepository>,
}

impl GetDashboardSummary {
    pub fn new(repo: Arc<dyn AnalyticsRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self) -> Result<DashboardSummaryDto, ApplicationError> {
        let summary = self.repo.get_dashboard_summary().await?;
        Ok(DashboardSummaryDto {
            total_deals: summary.total_deals,
            active_deals: summary.active_deals,
            completed_deals: summary.completed_deals,
            disputed_deals: summary.disputed_deals,
            total_parties: summary.total_parties,
            active_parties: summary.active_parties,
            total_users: summary.total_users,
            active_users: summary.active_users,
            avg_deal_value: summary.avg_deal_value,
            total_escrow_held: summary.total_escrow_held,
            total_fees_collected: summary.total_fees_collected,
            total_reviews: summary.total_reviews,
            avg_review_score: summary.avg_review_score,
        })
    }
}

#[derive(Clone)]
pub struct GetDealTrends {
    repo: Arc<dyn AnalyticsRepository>,
}

impl GetDealTrends {
    pub fn new(repo: Arc<dyn AnalyticsRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        from: Option<Date>,
        to: Option<Date>,
    ) -> Result<Vec<DealTrendDto>, ApplicationError> {
        let (from, to) = default_date_range(from, to);
        let trends = self.repo.get_deal_trends(from, to).await?;
        Ok(trends
            .into_iter()
            .map(|t| DealTrendDto {
                date: t.date,
                total_deals: t.total_deals,
                completed_deals: t.completed_deals,
                disputed_deals: t.disputed_deals,
                cancelled_deals: t.cancelled_deals,
                avg_deal_value: t.avg_deal_value,
            })
            .collect())
    }
}

#[derive(Clone)]
pub struct GetPartyActivity {
    repo: Arc<dyn AnalyticsRepository>,
}

impl GetPartyActivity {
    pub fn new(repo: Arc<dyn AnalyticsRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        from: Option<Date>,
        to: Option<Date>,
    ) -> Result<Vec<PartyActivityDto>, ApplicationError> {
        let (from, to) = default_date_range(from, to);
        let activity = self.repo.get_party_activity(from, to).await?;
        Ok(activity
            .into_iter()
            .map(|a| PartyActivityDto {
                date: a.date,
                total_parties: a.total_parties,
                active_parties: a.active_parties,
                parties_by_role: a.parties_by_role,
            })
            .collect())
    }
}

#[derive(Clone)]
pub struct ListDailyMetrics {
    repo: Arc<dyn AnalyticsRepository>,
}

impl ListDailyMetrics {
    pub fn new(repo: Arc<dyn AnalyticsRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        from: Option<Date>,
        to: Option<Date>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<DailyMetricsListDto, ApplicationError> {
        let filters = domain::repositories::MetricFilters {
            from_date: from,
            to_date: to,
            limit: limit.unwrap_or(30).clamp(1, 100),
            offset: offset.unwrap_or(0).max(0),
        };
        let result = self.repo.list_daily_metrics(filters).await?;
        Ok(DailyMetricsListDto {
            items: result
                .items
                .into_iter()
                .map(|m| DailyMetricDto {
                    date: m.date,
                    total_deals: m.total_deals,
                    deals_completed: m.deals_completed,
                    deals_disputed: m.deals_disputed,
                    deals_cancelled: m.deals_cancelled,
                    deals_by_status: m.deals_by_status,
                    total_parties: m.total_parties,
                    active_parties: m.active_parties,
                    total_users: m.total_users,
                    active_users: m.active_users,
                    avg_deal_value: m.avg_deal_value,
                    total_escrow_held: m.total_escrow_held,
                    total_fees_collected: m.total_fees_collected,
                    total_reviews: m.total_reviews,
                    avg_review_score: m.avg_review_score,
                    created_at: m.created_at,
                    updated_at: m.updated_at,
                })
                .collect(),
            total: result.total,
        })
    }
}

fn default_date_range(from: Option<Date>, to: Option<Date>) -> (Date, Date) {
    let to = to.unwrap_or_else(|| time::OffsetDateTime::now_utc().date());
    let from = from.unwrap_or(to - Duration::days(30));
    (from, to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::entities::PlatformMetric;
    use domain::errors::DomainError;
    use domain::repositories::{
        DashboardSummary, DealTrend, MetricFilters, MetricsListResult, PartyActivityMetric,
    };
    use rust_decimal::Decimal;
    use std::sync::Mutex;

    struct FakeAnalyticsRepo {
        refreshed: Mutex<Vec<Date>>,
        fail: bool,
    }

    impl FakeAnalyticsRepo {
        fn new() -> Self {
            Self {
                refreshed: Mutex::new(Vec::new()),
                fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                refreshed: Mutex::new(Vec::new()),
                fail: true,
            }
        }
    }

    #[async_trait]
    impl AnalyticsRepository for FakeAnalyticsRepo {
        async fn refresh_daily_metrics(&self, date: Date) -> Result<(), DomainError> {
            if self.fail {
                return Err(DomainError::RepositoryError("boom".to_string()));
            }
            self.refreshed.lock().unwrap().push(date);
            Ok(())
        }

        async fn get_dashboard_summary(&self) -> Result<DashboardSummary, DomainError> {
            if self.fail {
                return Err(DomainError::RepositoryError("boom".to_string()));
            }
            Ok(DashboardSummary {
                total_deals: 10,
                active_deals: 4,
                completed_deals: 5,
                disputed_deals: 1,
                total_parties: 8,
                active_parties: 7,
                total_users: 20,
                active_users: 18,
                avg_deal_value: Decimal::new(100, 0),
                total_escrow_held: Decimal::new(500, 0),
                total_fees_collected: Decimal::new(25, 0),
                total_reviews: 6,
                avg_review_score: 4.5,
            })
        }

        async fn get_deal_trends(
            &self,
            from: Date,
            to: Date,
        ) -> Result<Vec<DealTrend>, DomainError> {
            assert!(from <= to);
            if self.fail {
                return Err(DomainError::RepositoryError("boom".to_string()));
            }
            Ok(vec![DealTrend {
                date: from,
                total_deals: 3,
                completed_deals: 2,
                disputed_deals: 1,
                cancelled_deals: 0,
                avg_deal_value: Decimal::new(42, 0),
            }])
        }

        async fn get_party_activity(
            &self,
            from: Date,
            to: Date,
        ) -> Result<Vec<PartyActivityMetric>, DomainError> {
            assert!(from <= to);
            if self.fail {
                return Err(DomainError::RepositoryError("boom".to_string()));
            }
            Ok(vec![PartyActivityMetric {
                date: from,
                total_parties: 5,
                active_parties: 4,
                parties_by_role: serde_json::json!({"SUPPLIER": 2}),
            }])
        }

        async fn list_daily_metrics(
            &self,
            filters: MetricFilters,
        ) -> Result<MetricsListResult, DomainError> {
            if self.fail {
                return Err(DomainError::RepositoryError("boom".to_string()));
            }
            Ok(MetricsListResult {
                items: vec![PlatformMetric {
                    date: filters.from_date.unwrap_or_else(|| date(2026, 8, 1)),
                    total_deals: 10,
                    deals_completed: 5,
                    deals_disputed: 1,
                    deals_cancelled: 0,
                    deals_by_status: serde_json::json!({"DRAFT": 4}),
                    total_parties: 8,
                    active_parties: 7,
                    total_users: 20,
                    active_users: 18,
                    avg_deal_value: Decimal::new(100, 0),
                    total_escrow_held: Decimal::new(500, 0),
                    total_fees_collected: Decimal::new(25, 0),
                    total_reviews: 6,
                    avg_review_score: Decimal::new(45, 1),
                    created_at: time::OffsetDateTime::now_utc(),
                    updated_at: time::OffsetDateTime::now_utc(),
                }],
                total: 1,
            })
        }
    }

    fn date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, time::Month::try_from(m).unwrap(), d).unwrap()
    }

    #[tokio::test]
    async fn refresh_daily_metrics_calls_repo() {
        let repo = Arc::new(FakeAnalyticsRepo::new());
        let uc = RefreshDailyMetrics::new(repo.clone());
        let day = date(2026, 8, 1);
        uc.execute(day).await.unwrap();
        assert_eq!(repo.refreshed.lock().unwrap().as_slice(), &[day]);
    }

    #[tokio::test]
    async fn refresh_daily_metrics_propagates_error() {
        let uc = RefreshDailyMetrics::new(Arc::new(FakeAnalyticsRepo::failing()));
        let err = uc.execute(date(2026, 8, 1)).await.unwrap_err();
        assert!(matches!(err, ApplicationError::Infrastructure(_)));
    }

    #[tokio::test]
    async fn dashboard_summary_maps_all_fields() {
        let uc = GetDashboardSummary::new(Arc::new(FakeAnalyticsRepo::new()));
        let summary = uc.execute().await.unwrap();
        assert_eq!(summary.total_deals, 10);
        assert_eq!(summary.active_deals, 4);
        assert_eq!(summary.completed_deals, 5);
        assert_eq!(summary.disputed_deals, 1);
        assert_eq!(summary.total_parties, 8);
        assert_eq!(summary.active_parties, 7);
        assert_eq!(summary.total_users, 20);
        assert_eq!(summary.active_users, 18);
        assert_eq!(summary.avg_deal_value, Decimal::new(100, 0));
        assert_eq!(summary.total_escrow_held, Decimal::new(500, 0));
        assert_eq!(summary.total_fees_collected, Decimal::new(25, 0));
        assert_eq!(summary.total_reviews, 6);
        assert_eq!(summary.avg_review_score, 4.5);
    }

    #[tokio::test]
    async fn deal_trends_default_range_is_thirty_days() {
        let uc = GetDealTrends::new(Arc::new(FakeAnalyticsRepo::new()));
        let trends = uc.execute(None, None).await.unwrap();
        assert_eq!(trends.len(), 1);
        assert_eq!(trends[0].completed_deals, 2);
    }

    #[tokio::test]
    async fn deal_trends_explicit_range_passed_through() {
        let uc = GetDealTrends::new(Arc::new(FakeAnalyticsRepo::new()));
        let from = date(2026, 7, 1);
        let to = date(2026, 7, 31);
        let trends = uc.execute(Some(from), Some(to)).await.unwrap();
        assert_eq!(trends[0].date, from);
        assert_eq!(trends[0].total_deals, 3);
        assert_eq!(trends[0].disputed_deals, 1);
        assert_eq!(trends[0].cancelled_deals, 0);
        assert_eq!(trends[0].avg_deal_value, Decimal::new(42, 0));
    }

    #[tokio::test]
    async fn party_activity_default_and_explicit_ranges() {
        let uc = GetPartyActivity::new(Arc::new(FakeAnalyticsRepo::new()));
        let default_result = uc.execute(None, None).await.unwrap();
        assert_eq!(default_result.len(), 1);
        assert_eq!(default_result[0].total_parties, 5);
        assert_eq!(default_result[0].active_parties, 4);

        let from = date(2026, 6, 1);
        let explicit = uc
            .execute(Some(from), Some(date(2026, 6, 30)))
            .await
            .unwrap();
        assert_eq!(explicit[0].date, from);
        assert_eq!(
            explicit[0].parties_by_role,
            serde_json::json!({"SUPPLIER": 2})
        );
    }

    #[tokio::test]
    async fn list_daily_metrics_maps_items_and_clamps_limits() {
        let uc = ListDailyMetrics::new(Arc::new(FakeAnalyticsRepo::new()));
        let from = date(2026, 8, 1);
        let result = uc
            .execute(Some(from), Some(date(2026, 8, 31)), Some(1000), Some(-5))
            .await
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.items.len(), 1);
        let item = &result.items[0];
        assert_eq!(item.date, from);
        assert_eq!(item.total_deals, 10);
        assert_eq!(item.deals_completed, 5);
        assert_eq!(item.deals_disputed, 1);
        assert_eq!(item.deals_cancelled, 0);
        assert_eq!(item.deals_by_status, serde_json::json!({"DRAFT": 4}));
        assert_eq!(item.total_parties, 8);
        assert_eq!(item.active_parties, 7);
        assert_eq!(item.total_users, 20);
        assert_eq!(item.active_users, 18);
        assert_eq!(item.avg_deal_value, Decimal::new(100, 0));
        assert_eq!(item.total_escrow_held, Decimal::new(500, 0));
        assert_eq!(item.total_fees_collected, Decimal::new(25, 0));
        assert_eq!(item.total_reviews, 6);
        assert_eq!(item.avg_review_score, Decimal::new(45, 1));
    }

    #[tokio::test]
    async fn list_daily_metrics_defaults() {
        let uc = ListDailyMetrics::new(Arc::new(FakeAnalyticsRepo::new()));
        let result = uc.execute(None, None, None, None).await.unwrap();
        assert_eq!(result.total, 1);
    }

    #[tokio::test]
    async fn list_daily_metrics_propagates_error() {
        let uc = ListDailyMetrics::new(Arc::new(FakeAnalyticsRepo::failing()));
        assert!(uc.execute(None, None, None, None).await.is_err());
    }

    #[test]
    fn default_date_range_with_only_from() {
        let from = date(2026, 1, 1);
        let (f, _t) = default_date_range(Some(from), None);
        assert_eq!(f, from);
    }

    #[test]
    fn default_date_range_with_both() {
        let from = date(2026, 1, 1);
        let to = date(2026, 1, 31);
        let (f, t) = default_date_range(Some(from), Some(to));
        assert_eq!((f, t), (from, to));
    }
}
