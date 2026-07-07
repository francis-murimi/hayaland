use async_trait::async_trait;
use domain::entities::{
    DealRole, MatchGeneratedBy, MatchScoreBreakdown, MatchStatus, MatchSuggestion,
};
use domain::errors::DomainError;
use domain::repositories::{MatchCountByStatus, MatchFilters, MatchRepository};
use rust_decimal::prelude::{Decimal, FromPrimitive, ToPrimitive};
use sqlx::{Error as SqlxError, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresMatchRepository {
    pool: PgPool,
}

impl PostgresMatchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MatchRepository for PostgresMatchRepository {
    async fn create(&self, suggestion: &MatchSuggestion) -> Result<(), DomainError> {
        let score = Decimal::from_f64(suggestion.match_score).unwrap_or(Decimal::ZERO);
        let breakdown = serde_json::to_value(suggestion.score_breakdown)
            .map_err(|e| DomainError::RepositoryError(format!("JSON error: {e}")))?;
        let status = suggestion.match_status.as_str();
        let generated_by = suggestion.generated_by.as_str();

        sqlx::query!(
            r#"
            INSERT INTO match_suggestions (
                id, supplier_party_id, consumer_party_id, enhancer_party_id,
                match_status, match_score, score_breakdown, match_reason,
                resource_category_id, need_category_id, enhancement_category_id,
                suggested_deal_value, generated_by, expires_at, converted_deal_id,
                counter_notes, responded_at, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6::decimal, $7::jsonb, $8, $9, $10, $11,
                $12::decimal, $13, $14, $15, $16, $17, $18, $19
            )
            "#,
            suggestion.id,
            suggestion.supplier_party_id,
            suggestion.consumer_party_id,
            suggestion.enhancer_party_id,
            status,
            score,
            breakdown,
            suggestion.match_reason,
            suggestion.resource_category_id,
            suggestion.need_category_id,
            suggestion.enhancement_category_id,
            suggestion.suggested_deal_value,
            generated_by,
            suggestion.expires_at,
            suggestion.converted_deal_id,
            suggestion.counter_notes,
            suggestion.responded_at,
            suggestion.created_at,
            suggestion.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<MatchSuggestion>, DomainError> {
        let row = sqlx::query_as!(
            MatchSuggestionRow,
            r#"
            SELECT
                id, supplier_party_id, consumer_party_id, enhancer_party_id,
                match_status as "match_status!", match_score, score_breakdown, COALESCE(match_reason, '') as "match_reason!",
                resource_category_id, need_category_id, enhancement_category_id,
                suggested_deal_value, generated_by, expires_at, converted_deal_id,
                counter_notes, responded_at, created_at, updated_at
            FROM match_suggestions
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        row.map(build_match_from_row).transpose()
    }

    async fn list_for_party(
        &self,
        party_id: Uuid,
        role: Option<DealRole>,
        filters: &MatchFilters,
    ) -> Result<Vec<MatchSuggestion>, DomainError> {
        let status_filter = filters.status.as_ref().map(|s| s.as_str().to_string());
        let generated_by_filter = filters.generated_by.as_ref().map(|s| s.to_string());
        let role_filter = role.map(|r| r.as_str().to_string());

        let rows = sqlx::query_as!(
            MatchSuggestionRow,
            r#"
            SELECT
                id, supplier_party_id, consumer_party_id, enhancer_party_id,
                match_status as "match_status!", match_score, score_breakdown, COALESCE(match_reason, '') as "match_reason!",
                resource_category_id, need_category_id, enhancement_category_id,
                suggested_deal_value, generated_by, expires_at, converted_deal_id,
                counter_notes, responded_at, created_at, updated_at
            FROM match_suggestions
            WHERE (supplier_party_id = $1 OR consumer_party_id = $1 OR enhancer_party_id = $1)
            AND (
                $2::text IS NULL
                OR ($2 = 'SUPPLIER' AND supplier_party_id = $1)
                OR ($2 = 'CONSUMER' AND consumer_party_id = $1)
                OR ($2 = 'ENHANCER' AND enhancer_party_id = $1)
            )
            AND ($3::text IS NULL OR match_status = $3)
            AND ($4::decimal IS NULL OR match_score >= $4)
            AND ($5::decimal IS NULL OR match_score <= $5)
            AND ($6::text IS NULL OR generated_by = $6)
            AND ($7::timestamptz IS NULL OR created_at >= $7)
            AND ($8::timestamptz IS NULL OR created_at <= $8)
            ORDER BY match_score DESC, created_at DESC
            LIMIT $9 OFFSET $10
            "#,
            party_id,
            role_filter,
            status_filter,
            filters.min_score.and_then(Decimal::from_f64),
            filters.max_score.and_then(Decimal::from_f64),
            generated_by_filter,
            filters.created_after,
            filters.created_before,
            if filters.limit > 0 { filters.limit } else { i64::MAX },
            filters.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        rows.into_iter().map(build_match_from_row).collect()
    }

    async fn list_all(&self, filters: &MatchFilters) -> Result<Vec<MatchSuggestion>, DomainError> {
        let status_filter = filters.status.as_ref().map(|s| s.as_str().to_string());
        let generated_by_filter = filters.generated_by.as_ref().map(|s| s.to_string());

        let rows = sqlx::query_as!(
            MatchSuggestionRow,
            r#"
            SELECT
                id, supplier_party_id, consumer_party_id, enhancer_party_id,
                match_status as "match_status!", match_score, score_breakdown, COALESCE(match_reason, '') as "match_reason!",
                resource_category_id, need_category_id, enhancement_category_id,
                suggested_deal_value, generated_by, expires_at, converted_deal_id,
                counter_notes, responded_at, created_at, updated_at
            FROM match_suggestions
            WHERE ($1::text IS NULL OR match_status = $1)
            AND ($2::decimal IS NULL OR match_score >= $2)
            AND ($3::decimal IS NULL OR match_score <= $3)
            AND ($4::text IS NULL OR generated_by = $4)
            AND ($5::timestamptz IS NULL OR created_at >= $5)
            AND ($6::timestamptz IS NULL OR created_at <= $6)
            ORDER BY match_score DESC, created_at DESC
            LIMIT $7 OFFSET $8
            "#,
            status_filter,
            filters.min_score.and_then(Decimal::from_f64),
            filters.max_score.and_then(Decimal::from_f64),
            generated_by_filter,
            filters.created_after,
            filters.created_before,
            if filters.limit > 0 { filters.limit } else { i64::MAX },
            filters.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        rows.into_iter().map(build_match_from_row).collect()
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: MatchStatus,
        notes: Option<String>,
    ) -> Result<(), DomainError> {
        let status_str = status.as_str();
        let responded_at = if matches!(
            status,
            MatchStatus::Accepted | MatchStatus::Declined | MatchStatus::CounterProposed
        ) {
            Some(OffsetDateTime::now_utc())
        } else {
            None
        };

        sqlx::query!(
            r#"
            UPDATE match_suggestions
            SET match_status = $1,
                counter_notes = COALESCE($2, counter_notes),
                responded_at = COALESCE($3, responded_at),
                updated_at = now()
            WHERE id = $4
            "#,
            status_str,
            notes,
            responded_at,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_counter_proposal(
        &self,
        id: Uuid,
        value: Option<Decimal>,
        notes: Option<String>,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE match_suggestions
            SET match_status = 'COUNTER_PROPOSED',
                suggested_deal_value = COALESCE($1::decimal, suggested_deal_value),
                counter_notes = $2,
                responded_at = now(),
                updated_at = now()
            WHERE id = $3
            "#,
            value,
            notes,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn set_converted_deal(&self, id: Uuid, deal_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE match_suggestions
            SET match_status = 'CONVERTED_TO_DEAL',
                converted_deal_id = $1,
                updated_at = now()
            WHERE id = $2
            "#,
            deal_id,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn delete_by_party(
        &self,
        party_id: Uuid,
        status: Option<MatchStatus>,
    ) -> Result<u64, DomainError> {
        let status_filter = status.as_ref().map(|s| s.as_str().to_string());

        let result = sqlx::query!(
            r#"
            DELETE FROM match_suggestions
            WHERE (supplier_party_id = $1 OR consumer_party_id = $1 OR enhancer_party_id = $1)
            AND ($2::text IS NULL OR match_status = $2)
            "#,
            party_id,
            status_filter
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(result.rows_affected())
    }

    async fn delete_all(&self) -> Result<u64, DomainError> {
        let result = sqlx::query!("DELETE FROM match_suggestions")
            .execute(&self.pool)
            .await
            .map_err(map_err)?;

        Ok(result.rows_affected())
    }

    async fn count_by_status(&self, party_id: Uuid) -> Result<MatchCountByStatus, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN match_status = 'PENDING' THEN 1 ELSE 0 END), 0) as "pending!",
                COALESCE(SUM(CASE WHEN match_status = 'ACCEPTED' THEN 1 ELSE 0 END), 0) as "accepted!",
                COALESCE(SUM(CASE WHEN match_status = 'DECLINED' THEN 1 ELSE 0 END), 0) as "declined!",
                COALESCE(SUM(CASE WHEN match_status = 'COUNTER_PROPOSED' THEN 1 ELSE 0 END), 0) as "counter_proposed!",
                COALESCE(SUM(CASE WHEN match_status = 'EXPIRED' THEN 1 ELSE 0 END), 0) as "expired!",
                COALESCE(SUM(CASE WHEN match_status = 'CONVERTED_TO_DEAL' THEN 1 ELSE 0 END), 0) as "converted_to_deal!"
            FROM match_suggestions
            WHERE supplier_party_id = $1 OR consumer_party_id = $1 OR enhancer_party_id = $1
            "#,
            party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(MatchCountByStatus {
            pending: row.pending,
            accepted: row.accepted,
            declined: row.declined,
            counter_proposed: row.counter_proposed,
            expired: row.expired,
            converted_to_deal: row.converted_to_deal,
        })
    }

    async fn count_all_by_status(&self) -> Result<MatchCountByStatus, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN match_status = 'PENDING' THEN 1 ELSE 0 END), 0) as "pending!",
                COALESCE(SUM(CASE WHEN match_status = 'ACCEPTED' THEN 1 ELSE 0 END), 0) as "accepted!",
                COALESCE(SUM(CASE WHEN match_status = 'DECLINED' THEN 1 ELSE 0 END), 0) as "declined!",
                COALESCE(SUM(CASE WHEN match_status = 'COUNTER_PROPOSED' THEN 1 ELSE 0 END), 0) as "counter_proposed!",
                COALESCE(SUM(CASE WHEN match_status = 'EXPIRED' THEN 1 ELSE 0 END), 0) as "expired!",
                COALESCE(SUM(CASE WHEN match_status = 'CONVERTED_TO_DEAL' THEN 1 ELSE 0 END), 0) as "converted_to_deal!"
            FROM match_suggestions
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(MatchCountByStatus {
            pending: row.pending,
            accepted: row.accepted,
            declined: row.declined,
            counter_proposed: row.counter_proposed,
            expired: row.expired,
            converted_to_deal: row.converted_to_deal,
        })
    }

    async fn find_existing_pending(
        &self,
        supplier_party_id: Uuid,
        consumer_party_id: Uuid,
        enhancer_party_id: Uuid,
    ) -> Result<Option<MatchSuggestion>, DomainError> {
        let row = sqlx::query_as!(
            MatchSuggestionRow,
            r#"
            SELECT
                id, supplier_party_id, consumer_party_id, enhancer_party_id,
                match_status as "match_status!", match_score, score_breakdown, COALESCE(match_reason, '') as "match_reason!",
                resource_category_id, need_category_id, enhancement_category_id,
                suggested_deal_value, generated_by, expires_at, converted_deal_id,
                counter_notes, responded_at, created_at, updated_at
            FROM match_suggestions
            WHERE supplier_party_id = $1
              AND consumer_party_id = $2
              AND enhancer_party_id = $3
              AND match_status = 'PENDING'
            "#,
            supplier_party_id,
            consumer_party_id,
            enhancer_party_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        row.map(build_match_from_row).transpose()
    }
}

#[derive(sqlx::FromRow)]
struct MatchSuggestionRow {
    id: Uuid,
    supplier_party_id: Uuid,
    consumer_party_id: Uuid,
    enhancer_party_id: Uuid,
    match_status: String,
    match_score: Decimal,
    score_breakdown: serde_json::Value,
    match_reason: String,
    resource_category_id: Option<Uuid>,
    need_category_id: Option<Uuid>,
    enhancement_category_id: Option<Uuid>,
    suggested_deal_value: Option<Decimal>,
    generated_by: String,
    expires_at: Option<OffsetDateTime>,
    converted_deal_id: Option<Uuid>,
    counter_notes: Option<String>,
    responded_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn build_match_from_row(row: MatchSuggestionRow) -> Result<MatchSuggestion, DomainError> {
    let score_breakdown: MatchScoreBreakdown = serde_json::from_value(row.score_breakdown)
        .map_err(|e| DomainError::RepositoryError(format!("invalid score_breakdown JSON: {e}")))?;

    let suggestion = MatchSuggestion {
        id: row.id,
        supplier_party_id: row.supplier_party_id,
        consumer_party_id: row.consumer_party_id,
        enhancer_party_id: row.enhancer_party_id,
        match_status: MatchStatus::try_from(row.match_status.as_str())?,
        match_score: row.match_score.to_f64().unwrap_or(0.0),
        score_breakdown,
        match_reason: row.match_reason,
        resource_category_id: row.resource_category_id,
        need_category_id: row.need_category_id,
        enhancement_category_id: row.enhancement_category_id,
        suggested_deal_value: row.suggested_deal_value,
        generated_by: MatchGeneratedBy::try_from(row.generated_by.as_str())?,
        expires_at: row.expires_at,
        converted_deal_id: row.converted_deal_id,
        counter_notes: row.counter_notes,
        responded_at: row.responded_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(suggestion)
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
