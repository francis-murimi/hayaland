use async_trait::async_trait;
use domain::entities::PlatformMetric;
use domain::errors::DomainError;
use domain::repositories::{
    AnalyticsRepository, DashboardSummary, DealTrend, MetricFilters, MetricsListResult,
    PartyActivityMetric,
};
use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, PgPool, Row};
use time::Date;

pub struct PostgresAnalyticsRepository {
    pool: PgPool,
}

impl PostgresAnalyticsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AnalyticsRepository for PostgresAnalyticsRepository {
    async fn refresh_daily_metrics(&self, date: Date) -> Result<(), DomainError> {
        let total_deals: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM deals WHERE created_at::DATE <= $1"#,
        )
        .bind(date)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        if total_deals == 0 {
            return Err(DomainError::RepositoryError(
                "no metrics refreshed".to_string(),
            ));
        }

        let result = sqlx::query(
            r#"
            WITH filtered_deals AS (
                SELECT * FROM deals WHERE created_at::DATE <= $1
            ),
            status_counts AS (
                SELECT deal_status::TEXT AS status, COUNT(*)::INTEGER AS cnt
                FROM filtered_deals
                GROUP BY deal_status
            )
            INSERT INTO platform_metrics (
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
                avg_review_score
            )
            SELECT
                $1,
                (SELECT COUNT(*)::INTEGER FROM filtered_deals),
                (SELECT COUNT(*)::INTEGER FROM filtered_deals WHERE deal_status = 'COMPLETED'),
                (SELECT COUNT(*)::INTEGER FROM filtered_deals WHERE deal_status = 'DISPUTED'),
                (SELECT COUNT(*)::INTEGER FROM filtered_deals WHERE deal_status IN ('CANCELLED', 'EXPIRED')),
                COALESCE((SELECT jsonb_object_agg(status, cnt) FROM status_counts), '{}'),
                (SELECT COUNT(*)::INTEGER FROM parties),
                (SELECT COUNT(*)::INTEGER FROM parties WHERE is_active = true),
                (SELECT COUNT(*)::INTEGER FROM users),
                (SELECT COUNT(*)::INTEGER FROM users WHERE is_active = true),
                COALESCE((SELECT AVG(total_deal_value) FROM filtered_deals WHERE total_deal_value IS NOT NULL), 0),
                COALESCE((SELECT SUM(escrow_balance) FROM platform_wallets), 0),
                COALESCE((SELECT SUM(amount) FROM transactions WHERE transaction_type = 'FEE'), 0),
                (SELECT COUNT(*)::INTEGER FROM reviews),
                COALESCE((SELECT AVG(overall_rating)::DECIMAL(3,2) FROM reviews), 0)
            FROM (SELECT 1) AS dummy
            ON CONFLICT (date) DO UPDATE SET
                total_deals = EXCLUDED.total_deals,
                deals_completed = EXCLUDED.deals_completed,
                deals_disputed = EXCLUDED.deals_disputed,
                deals_cancelled = EXCLUDED.deals_cancelled,
                deals_by_status = EXCLUDED.deals_by_status,
                total_parties = EXCLUDED.total_parties,
                active_parties = EXCLUDED.active_parties,
                total_users = EXCLUDED.total_users,
                active_users = EXCLUDED.active_users,
                avg_deal_value = EXCLUDED.avg_deal_value,
                total_escrow_held = EXCLUDED.total_escrow_held,
                total_fees_collected = EXCLUDED.total_fees_collected,
                total_reviews = EXCLUDED.total_reviews,
                avg_review_score = EXCLUDED.avg_review_score,
                updated_at = now()
            "#,
        )
        .bind(date)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::RepositoryError(
                "no metrics refreshed".to_string(),
            ));
        }

        Ok(())
    }

    async fn get_dashboard_summary(&self) -> Result<DashboardSummary, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT
                total_deals::BIGINT AS total_deals,
                (total_deals - deals_completed - deals_cancelled - deals_disputed)::BIGINT AS active_deals,
                deals_completed::BIGINT AS deals_completed,
                deals_disputed::BIGINT AS deals_disputed,
                total_parties::BIGINT AS total_parties,
                active_parties::BIGINT AS active_parties,
                total_users::BIGINT AS total_users,
                active_users::BIGINT AS active_users,
                avg_deal_value,
                total_escrow_held,
                total_fees_collected,
                total_reviews::BIGINT AS total_reviews,
                avg_review_score::DOUBLE PRECISION AS avg_review_score
            FROM platform_metrics
            ORDER BY date DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        let summary = match row {
            Some(row) => DashboardSummary {
                total_deals: row.try_get("total_deals").unwrap_or(0),
                active_deals: row.try_get("active_deals").unwrap_or(0),
                completed_deals: row.try_get("deals_completed").unwrap_or(0),
                disputed_deals: row.try_get("deals_disputed").unwrap_or(0),
                total_parties: row.try_get("total_parties").unwrap_or(0),
                active_parties: row.try_get("active_parties").unwrap_or(0),
                total_users: row.try_get("total_users").unwrap_or(0),
                active_users: row.try_get("active_users").unwrap_or(0),
                avg_deal_value: row.try_get("avg_deal_value").unwrap_or(Decimal::ZERO),
                total_escrow_held: row.try_get("total_escrow_held").unwrap_or(Decimal::ZERO),
                total_fees_collected: row.try_get("total_fees_collected").unwrap_or(Decimal::ZERO),
                total_reviews: row.try_get("total_reviews").unwrap_or(0),
                avg_review_score: row.try_get("avg_review_score").unwrap_or(0.0),
            },
            None => DashboardSummary::default(),
        };

        Ok(summary)
    }

    async fn get_deal_trends(&self, from: Date, to: Date) -> Result<Vec<DealTrend>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT
                date,
                total_deals::BIGINT AS total_deals,
                deals_completed::BIGINT AS deals_completed,
                deals_disputed::BIGINT AS deals_disputed,
                deals_cancelled::BIGINT AS deals_cancelled,
                avg_deal_value
            FROM platform_metrics
            WHERE date BETWEEN $1 AND $2
            ORDER BY date ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows
            .into_iter()
            .map(|row| DealTrend {
                date: row.try_get("date").unwrap(),
                total_deals: row.try_get("total_deals").unwrap_or(0),
                completed_deals: row.try_get("deals_completed").unwrap_or(0),
                disputed_deals: row.try_get("deals_disputed").unwrap_or(0),
                cancelled_deals: row.try_get("deals_cancelled").unwrap_or(0),
                avg_deal_value: row.try_get("avg_deal_value").unwrap_or(Decimal::ZERO),
            })
            .collect())
    }

    async fn get_party_activity(
        &self,
        from: Date,
        to: Date,
    ) -> Result<Vec<PartyActivityMetric>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT
                date,
                total_parties::BIGINT AS total_parties,
                active_parties::BIGINT AS active_parties,
                '{}'::JSONB AS parties_by_role
            FROM platform_metrics
            WHERE date BETWEEN $1 AND $2
            ORDER BY date ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows
            .into_iter()
            .map(|row| PartyActivityMetric {
                date: row.try_get("date").unwrap(),
                total_parties: row.try_get("total_parties").unwrap_or(0),
                active_parties: row.try_get("active_parties").unwrap_or(0),
                parties_by_role: row
                    .try_get("parties_by_role")
                    .unwrap_or(serde_json::Value::Null),
            })
            .collect())
    }

    async fn list_daily_metrics(
        &self,
        filters: MetricFilters,
    ) -> Result<MetricsListResult, DomainError> {
        let rows = sqlx::query_as!(
            PlatformMetricRow,
            r#"
            SELECT
                date,
                total_deals,
                deals_completed,
                deals_disputed,
                deals_cancelled,
                deals_by_status as "deals_by_status!: sqlx::types::Json<serde_json::Value>",
                total_parties,
                active_parties,
                total_users,
                active_users,
                avg_deal_value,
                total_escrow_held,
                total_fees_collected,
                total_reviews,
                avg_review_score,
                created_at,
                updated_at
            FROM platform_metrics
            WHERE ($1::DATE IS NULL OR date >= $1)
              AND ($2::DATE IS NULL OR date <= $2)
            ORDER BY date DESC
            LIMIT $3 OFFSET $4
            "#,
            filters.from_date,
            filters.to_date,
            filters.limit,
            filters.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM platform_metrics
            WHERE ($1::DATE IS NULL OR date >= $1)
              AND ($2::DATE IS NULL OR date <= $2)
            "#,
            filters.from_date,
            filters.to_date
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(MetricsListResult {
            items: rows.into_iter().map(build_platform_metric).collect(),
            total,
        })
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}

#[derive(Debug, sqlx::FromRow)]
struct PlatformMetricRow {
    date: Date,
    total_deals: i32,
    deals_completed: i32,
    deals_disputed: i32,
    deals_cancelled: i32,
    deals_by_status: sqlx::types::Json<serde_json::Value>,
    total_parties: i32,
    active_parties: i32,
    total_users: i32,
    active_users: i32,
    avg_deal_value: Decimal,
    total_escrow_held: Decimal,
    total_fees_collected: Decimal,
    total_reviews: i32,
    avg_review_score: Decimal,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn build_platform_metric(row: PlatformMetricRow) -> PlatformMetric {
    PlatformMetric {
        date: row.date,
        total_deals: row.total_deals,
        deals_completed: row.deals_completed,
        deals_disputed: row.deals_disputed,
        deals_cancelled: row.deals_cancelled,
        deals_by_status: row.deals_by_status.0,
        total_parties: row.total_parties,
        active_parties: row.active_parties,
        total_users: row.total_users,
        active_users: row.active_users,
        avg_deal_value: row.avg_deal_value,
        total_escrow_held: row.total_escrow_held,
        total_fees_collected: row.total_fees_collected,
        total_reviews: row.total_reviews,
        avg_review_score: row.avg_review_score,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}
