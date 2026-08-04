use serde::{Deserialize, Serialize};
use time::Date;

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSummaryDto {
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

#[derive(Debug, Clone, Serialize)]
pub struct DealTrendDto {
    pub date: Date,
    pub total_deals: i64,
    pub completed_deals: i64,
    pub disputed_deals: i64,
    pub cancelled_deals: i64,
    pub avg_deal_value: rust_decimal::Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct PartyActivityDto {
    pub date: Date,
    pub total_parties: i64,
    pub active_parties: i64,
    pub parties_by_role: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DateRangeQuery {
    pub from: Option<Date>,
    pub to: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyMetricDto {
    pub date: Date,
    pub total_deals: i32,
    pub deals_completed: i32,
    pub deals_disputed: i32,
    pub deals_cancelled: i32,
    pub deals_by_status: serde_json::Value,
    pub total_parties: i32,
    pub active_parties: i32,
    pub total_users: i32,
    pub active_users: i32,
    pub avg_deal_value: rust_decimal::Decimal,
    pub total_escrow_held: rust_decimal::Decimal,
    pub total_fees_collected: rust_decimal::Decimal,
    pub total_reviews: i32,
    pub avg_review_score: rust_decimal::Decimal,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyMetricsListDto {
    pub items: Vec<DailyMetricDto>,
    pub total: i64,
}
