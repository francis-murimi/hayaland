use crate::entities::PlatformMetric;
use crate::errors::DomainError;
use async_trait::async_trait;
use time::Date;

/// A lightweight summary returned for the admin dashboard.
#[derive(Debug, Clone, Default)]
pub struct DashboardSummary {
    pub total_deals: i64,
    pub active_deals: i64,
    pub completed_deals: i64,
    pub disputed_deals: i64,
    pub total_parties: i64,
    pub active_parties: i64,
    pub total_users: i64,
    pub active_users: i64,
    pub avg_deal_value: rust_decimal::Decimal,
    pub total_escrow_held: rust_decimal::Decimal,
    pub total_fees_collected: rust_decimal::Decimal,
    pub total_reviews: i64,
    pub avg_review_score: f64,
}

/// A single point on a deal trend line.
#[derive(Debug, Clone)]
pub struct DealTrend {
    pub date: Date,
    pub total_deals: i64,
    pub completed_deals: i64,
    pub disputed_deals: i64,
    pub cancelled_deals: i64,
    pub avg_deal_value: rust_decimal::Decimal,
}

/// A single point on a party activity trend line.
#[derive(Debug, Clone)]
pub struct PartyActivityMetric {
    pub date: Date,
    pub total_parties: i64,
    pub active_parties: i64,
    pub parties_by_role: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct MetricsListResult {
    pub items: Vec<PlatformMetric>,
    pub total: i64,
}

#[derive(Debug, Clone, Default)]
pub struct MetricFilters {
    pub from_date: Option<Date>,
    pub to_date: Option<Date>,
    pub limit: i64,
    pub offset: i64,
}

/// Outbound port for platform analytics and reporting.
#[async_trait]
pub trait AnalyticsRepository: Send + Sync {
    /// Compute and upsert a daily snapshot for the given date.
    async fn refresh_daily_metrics(&self, date: Date) -> Result<(), DomainError>;

    /// Load the most recent dashboard summary.
    async fn get_dashboard_summary(&self) -> Result<DashboardSummary, DomainError>;

    /// Return daily deal trends between two dates.
    async fn get_deal_trends(&self, from: Date, to: Date) -> Result<Vec<DealTrend>, DomainError>;

    /// Return daily party activity between two dates.
    async fn get_party_activity(
        &self,
        from: Date,
        to: Date,
    ) -> Result<Vec<PartyActivityMetric>, DomainError>;

    /// List stored daily metrics with optional date filtering.
    async fn list_daily_metrics(
        &self,
        filters: MetricFilters,
    ) -> Result<MetricsListResult, DomainError>;
}
