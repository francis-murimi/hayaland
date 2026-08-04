use serde::{Deserialize, Serialize};
use time::Date;

/// A daily snapshot of platform-wide metrics used by analytics dashboards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformMetric {
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

impl PlatformMetric {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        date: Date,
        total_deals: i32,
        deals_completed: i32,
        deals_disputed: i32,
        deals_cancelled: i32,
        deals_by_status: serde_json::Value,
        total_parties: i32,
        active_parties: i32,
        total_users: i32,
        active_users: i32,
        avg_deal_value: rust_decimal::Decimal,
        total_escrow_held: rust_decimal::Decimal,
        total_fees_collected: rust_decimal::Decimal,
        total_reviews: i32,
        avg_review_score: rust_decimal::Decimal,
    ) -> Self {
        let now = time::OffsetDateTime::now_utc();
        Self {
            date,
            total_deals,
            deals_completed,
            deals_disputed,
            deals_cancelled,
            deals_by_status,
            total_parties,
            active_parties,
            total_users,
            active_users,
            avg_deal_value,
            total_escrow_held,
            total_fees_collected,
            total_reviews,
            avg_review_score,
            created_at: now,
            updated_at: now,
        }
    }
}
