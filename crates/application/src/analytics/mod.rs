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
