use async_trait::async_trait;
use domain::entities::trust_score::{
    DisputeInput, ResponseMetrics, ReviewInput, RoleDealInput, TrustScoreRow,
};
use domain::errors::DomainError;
use domain::repositories::TrustScoreRepository;
use sqlx::{Error as SqlxError, PgPool};
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresTrustScoreRepository {
    pool: PgPool,
}

impl PostgresTrustScoreRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TrustScoreRepository for PostgresTrustScoreRepository {
    async fn find_by_party_id(&self, party_id: Uuid) -> Result<Option<TrustScoreRow>, DomainError> {
        let row = sqlx::query_as!(
            TrustScoreRowDb,
            r#"
            SELECT
                id,
                party_id,
                overall_score as "overall_score!: f64",
                as_supplier_score,
                as_consumer_score,
                as_enhancer_score,
                deals_completed_count as "deals_completed_count!: i64",
                deals_cancelled_count as "deals_cancelled_count!: i64",
                deals_disputed_count as "deals_disputed_count!: i64",
                timeouts_count as "timeouts_count!: i64",
                no_shows_count as "no_shows_count!: i64",
                total_completed_value as "total_completed_value!: f64",
                average_response_hours,
                profile_completeness as "profile_completeness!: f64",
                verification_level as "verification_level!: i32",
                longevity_days as "longevity_days!: i64",
                calculation_formula as "calculation_formula!: serde_json::Value",
                last_calculated_at,
                next_calculation_at
            FROM trust_scores
            WHERE party_id = $1
            "#,
            party_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(Into::into))
    }

    async fn upsert(&self, row: &TrustScoreRow) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO trust_scores (
                id, party_id, overall_score, as_supplier_score, as_consumer_score, as_enhancer_score,
                deals_completed_count, deals_cancelled_count, deals_disputed_count,
                timeouts_count, no_shows_count, total_completed_value,
                average_response_hours, profile_completeness, verification_level, longevity_days,
                calculation_formula, last_calculated_at, next_calculation_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, now(), now())
            ON CONFLICT (party_id) DO UPDATE SET
                overall_score = EXCLUDED.overall_score,
                as_supplier_score = EXCLUDED.as_supplier_score,
                as_consumer_score = EXCLUDED.as_consumer_score,
                as_enhancer_score = EXCLUDED.as_enhancer_score,
                deals_completed_count = EXCLUDED.deals_completed_count,
                deals_cancelled_count = EXCLUDED.deals_cancelled_count,
                deals_disputed_count = EXCLUDED.deals_disputed_count,
                timeouts_count = EXCLUDED.timeouts_count,
                no_shows_count = EXCLUDED.no_shows_count,
                total_completed_value = EXCLUDED.total_completed_value,
                average_response_hours = EXCLUDED.average_response_hours,
                profile_completeness = EXCLUDED.profile_completeness,
                verification_level = EXCLUDED.verification_level,
                longevity_days = EXCLUDED.longevity_days,
                calculation_formula = EXCLUDED.calculation_formula,
                last_calculated_at = EXCLUDED.last_calculated_at,
                next_calculation_at = EXCLUDED.next_calculation_at,
                updated_at = now()
            "#,
            row.id,
            row.party_id,
            row.overall_score,
            row.as_supplier_score,
            row.as_consumer_score,
            row.as_enhancer_score,
            row.deals_completed_count,
            row.deals_cancelled_count,
            row.deals_disputed_count,
            row.timeouts_count,
            row.no_shows_count,
            row.total_completed_value,
            row.average_response_hours,
            row.profile_completeness,
            row.verification_level,
            row.longevity_days,
            row.calculation_formula,
            row.last_calculated_at,
            row.next_calculation_at,
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn increment_deals_completed_count(
        &self,
        party_id: Uuid,
        deal_value: f64,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO trust_scores (
                id, party_id, deals_completed_count, deals_cancelled_count, deals_disputed_count,
                timeouts_count, no_shows_count, total_completed_value
            )
            VALUES ($1, $2, 1, 0, 0, 0, 0, $3)
            ON CONFLICT (party_id) DO UPDATE SET
                deals_completed_count = trust_scores.deals_completed_count + 1,
                total_completed_value = trust_scores.total_completed_value + EXCLUDED.total_completed_value,
                updated_at = now()
            "#,
            Uuid::now_v7(),
            party_id,
            deal_value,
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn increment_deals_cancelled_count(&self, party_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO trust_scores (
                id, party_id, deals_completed_count, deals_cancelled_count, deals_disputed_count,
                timeouts_count, no_shows_count, total_completed_value
            )
            VALUES ($1, $2, 0, 1, 0, 0, 0, 0)
            ON CONFLICT (party_id) DO UPDATE SET
                deals_cancelled_count = trust_scores.deals_cancelled_count + 1,
                updated_at = now()
            "#,
            Uuid::now_v7(),
            party_id,
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn increment_deals_disputed_count(&self, party_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO trust_scores (
                id, party_id, deals_completed_count, deals_cancelled_count, deals_disputed_count,
                timeouts_count, no_shows_count, total_completed_value
            )
            VALUES ($1, $2, 0, 0, 1, 0, 0, 0)
            ON CONFLICT (party_id) DO UPDATE SET
                deals_disputed_count = trust_scores.deals_disputed_count + 1,
                updated_at = now()
            "#,
            Uuid::now_v7(),
            party_id,
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn increment_timeouts_count(&self, party_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO trust_scores (
                id, party_id, deals_completed_count, deals_cancelled_count, deals_disputed_count,
                timeouts_count, no_shows_count, total_completed_value
            )
            VALUES ($1, $2, 0, 0, 0, 1, 0, 0)
            ON CONFLICT (party_id) DO UPDATE SET
                timeouts_count = trust_scores.timeouts_count + 1,
                updated_at = now()
            "#,
            Uuid::now_v7(),
            party_id,
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn increment_no_shows_count(&self, party_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO trust_scores (
                id, party_id, deals_completed_count, deals_cancelled_count, deals_disputed_count,
                timeouts_count, no_shows_count, total_completed_value
            )
            VALUES ($1, $2, 0, 0, 0, 0, 1, 0)
            ON CONFLICT (party_id) DO UPDATE SET
                no_shows_count = trust_scores.no_shows_count + 1,
                updated_at = now()
            "#,
            Uuid::now_v7(),
            party_id,
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_profile_completeness(
        &self,
        party_id: Uuid,
        completeness: f64,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO trust_scores (id, party_id, profile_completeness)
            VALUES ($1, $2, $3)
            ON CONFLICT (party_id) DO UPDATE SET
                profile_completeness = EXCLUDED.profile_completeness,
                updated_at = now()
            "#,
            Uuid::now_v7(),
            party_id,
            completeness,
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_verification_level(
        &self,
        party_id: Uuid,
        level: i32,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO trust_scores (
                id, party_id, overall_score, deals_completed_count, deals_cancelled_count,
                deals_disputed_count, profile_completeness, verification_level, longevity_days,
                calculation_formula, created_at, updated_at
            )
            VALUES ($1, $2, 0, 0, 0, 0, 0, $3, 0, '{}', now(), now())
            ON CONFLICT (party_id) DO UPDATE SET
                verification_level = EXCLUDED.verification_level,
                updated_at = now()
            "#,
            Uuid::now_v7(),
            party_id,
            level,
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_response_hours(
        &self,
        party_id: Uuid,
        hours: Option<f64>,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO trust_scores (id, party_id, average_response_hours)
            VALUES ($1, $2, $3)
            ON CONFLICT (party_id) DO UPDATE SET
                average_response_hours = EXCLUDED.average_response_hours,
                updated_at = now()
            "#,
            Uuid::now_v7(),
            party_id,
            hours,
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn create_default(&self, party_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO trust_scores (
                id, party_id, overall_score, deals_completed_count, deals_cancelled_count,
                deals_disputed_count, timeouts_count, no_shows_count, total_completed_value,
                profile_completeness, verification_level, longevity_days, calculation_formula,
                created_at, updated_at
            )
            VALUES ($1, $2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, '{}', now(), now())
            ON CONFLICT (party_id) DO NOTHING
            "#,
            Uuid::now_v7(),
            party_id,
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_public_cache(&self, party_id: Uuid, score: f64) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE parties
            SET trust_score = $1,
                updated_at = now()
            WHERE id = $2
            "#,
            score,
            party_id,
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list_party_ids(&self, limit: i64, offset: i64) -> Result<Vec<Uuid>, DomainError> {
        let rows = sqlx::query_scalar!(
            r#"
            SELECT id as "id!"
            FROM parties
            ORDER BY created_at ASC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows)
    }

    async fn compute_response_metrics(
        &self,
        party_id: Uuid,
    ) -> Result<ResponseMetrics, DomainError> {
        let window = OffsetDateTime::now_utc() - time::Duration::days(90);

        let received = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM messages
            WHERE recipient_party_id = $1
              AND created_at > $2
              AND message_type = 'TEXT'
            "#,
            party_id,
            window,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let row = sqlx::query!(
            r#"
            SELECT
                COALESCE(AVG(EXTRACT(EPOCH FROM (mr.read_at - m.created_at)) / 3600.0), 0.0) as "avg_hours!: f64",
                COUNT(*) as "responded!"
            FROM messages m
            JOIN message_reads mr ON mr.message_id = m.id
            WHERE m.recipient_party_id = $1
              AND m.created_at > $2
              AND m.message_type = 'TEXT'
            "#,
            party_id,
            window,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(ResponseMetrics {
            average_response_hours: if row.responded > 0 {
                Some(row.avg_hours)
            } else {
                None
            },
            messages_received_90d: received,
            messages_responded_90d: row.responded,
        })
    }

    async fn find_role_deal_inputs(
        &self,
        party_id: Uuid,
    ) -> Result<HashMap<String, RoleDealInput>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                dp.role as "role!",
                COUNT(*) FILTER (WHERE d.deal_status = 'COMPLETED') as "completed!: i64",
                COUNT(*) FILTER (WHERE d.deal_status = 'CANCELLED') as "cancelled!: i64",
                COALESCE(SUM(d.total_deal_value) FILTER (WHERE d.deal_status = 'COMPLETED'), 0.0) as "value!: f64"
            FROM deal_participations dp
            JOIN deals d ON d.id = dp.deal_id
            WHERE dp.party_id = $1
            GROUP BY dp.role
            "#,
            party_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.role,
                    RoleDealInput {
                        deals_completed_count: r.completed,
                        deals_cancelled_count: r.cancelled,
                        total_completed_value: r.value,
                    },
                )
            })
            .collect())
    }

    async fn find_role_reviews(
        &self,
        party_id: Uuid,
    ) -> Result<HashMap<String, Vec<ReviewInput>>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                r.reviewed_role as "reviewed_role!",
                r.reviewer_party_id as "reviewer_party_id!",
                r.overall_rating as "overall_rating!: i32",
                r.communication_rating,
                r.reliability_rating,
                r.quality_rating,
                r.timeliness_rating,
                COALESCE(d.total_deal_value, 0.0) as "deal_value!: f64",
                r.created_at as "created_at!: OffsetDateTime",
                r.is_public as "is_public!: bool",
                p.trust_score as "reviewer_trust_score!: f64"
            FROM reviews r
            LEFT JOIN deals d ON d.id = r.deal_id
            LEFT JOIN parties p ON p.id = r.reviewer_party_id
            WHERE r.reviewed_party_id = $1
              AND r.is_public = true
            ORDER BY r.created_at DESC
            "#,
            party_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let mut map: HashMap<String, Vec<ReviewInput>> = HashMap::new();
        for r in rows {
            let dims = [
                r.communication_rating,
                r.reliability_rating,
                r.quality_rating,
                r.timeliness_rating,
            ]
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();

            let review_score = if dims.is_empty() {
                r.overall_rating as f64
            } else {
                dims.iter().sum::<i32>() as f64 / dims.len() as f64
            };

            let input = ReviewInput {
                reviewer_party_id: Some(r.reviewer_party_id),
                reviewer_overall_score: r.reviewer_trust_score,
                review_score,
                deal_value: r.deal_value,
                created_at: r.created_at,
                is_public: r.is_public,
                is_hidden: false,
            };

            map.entry(r.reviewed_role).or_default().push(input);
        }

        Ok(map)
    }

    async fn find_dispute_inputs(&self, party_id: Uuid) -> Result<Vec<DisputeInput>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                raised_by_party_id as "raised_by_party_id!",
                against_party_id,
                resolution_type,
                resolution_outcome,
                created_at as "created_at!: OffsetDateTime",
                updated_at as "updated_at!: OffsetDateTime"
            FROM disputes
            WHERE raised_by_party_id = $1 OR against_party_id = $1
            ORDER BY created_at DESC
            "#,
            party_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows
            .into_iter()
            .map(|r| DisputeInput {
                raised_by_party_id: r.raised_by_party_id,
                against_party_id: r.against_party_id,
                resolution_type: r.resolution_type,
                resolution_outcome: r.resolution_outcome,
                created_at: r.created_at,
                resolved_at: Some(r.updated_at),
            })
            .collect())
    }

    async fn find_review_inputs(&self, party_id: Uuid) -> Result<Vec<ReviewInput>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                r.reviewer_party_id as "reviewer_party_id!",
                r.overall_rating as "overall_rating!: i32",
                r.communication_rating,
                r.reliability_rating,
                r.quality_rating,
                r.timeliness_rating,
                COALESCE(d.total_deal_value, 0.0) as "deal_value!: f64",
                r.created_at as "created_at!: OffsetDateTime",
                r.is_public as "is_public!: bool",
                p.trust_score as "reviewer_trust_score!: f64"
            FROM reviews r
            LEFT JOIN deals d ON d.id = r.deal_id
            LEFT JOIN parties p ON p.id = r.reviewer_party_id
            WHERE r.reviewed_party_id = $1
            ORDER BY r.created_at DESC
            "#,
            party_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let dims = [
                    r.communication_rating,
                    r.reliability_rating,
                    r.quality_rating,
                    r.timeliness_rating,
                ]
                .iter()
                .flatten()
                .copied()
                .collect::<Vec<_>>();

                let review_score = if dims.is_empty() {
                    r.overall_rating as f64
                } else {
                    dims.iter().sum::<i32>() as f64 / dims.len() as f64
                };

                ReviewInput {
                    reviewer_party_id: Some(r.reviewer_party_id),
                    reviewer_overall_score: r.reviewer_trust_score,
                    review_score,
                    deal_value: r.deal_value,
                    created_at: r.created_at,
                    is_public: r.is_public,
                    is_hidden: !r.is_public,
                }
            })
            .collect())
    }

    async fn find_account_age_and_activity(
        &self,
        party_id: Uuid,
    ) -> Result<(i64, Option<i64>), DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT
                created_at as "created_at!: OffsetDateTime",
                updated_at as "updated_at!: OffsetDateTime"
            FROM parties
            WHERE id = $1
            "#,
            party_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let now = OffsetDateTime::now_utc();
        let age_days = (now - row.created_at).whole_days();

        let last_activity = sqlx::query_scalar!(
            r#"
            SELECT MAX(created_at) as "created_at"
            FROM (
                SELECT created_at FROM messages WHERE sender_party_id = $1 OR recipient_party_id = $1
                UNION ALL
                SELECT created_at FROM deal_history WHERE actor_party_id = $1
                UNION ALL
                SELECT created_at FROM reviews WHERE reviewed_party_id = $1 OR reviewer_party_id = $1
            ) activity
            "#,
            party_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let days_since_activity = last_activity.map(|dt| (now - dt).whole_days());

        Ok((age_days, days_since_activity))
    }
}

#[derive(Debug, Clone)]
struct TrustScoreRowDb {
    id: Uuid,
    party_id: Uuid,
    overall_score: f64,
    as_supplier_score: Option<f64>,
    as_consumer_score: Option<f64>,
    as_enhancer_score: Option<f64>,
    deals_completed_count: i64,
    deals_cancelled_count: i64,
    deals_disputed_count: i64,
    timeouts_count: i64,
    no_shows_count: i64,
    total_completed_value: f64,
    average_response_hours: Option<f64>,
    profile_completeness: f64,
    verification_level: i32,
    longevity_days: i64,
    calculation_formula: serde_json::Value,
    last_calculated_at: Option<OffsetDateTime>,
    next_calculation_at: Option<OffsetDateTime>,
}

impl From<TrustScoreRowDb> for TrustScoreRow {
    fn from(db: TrustScoreRowDb) -> Self {
        Self {
            id: db.id,
            party_id: db.party_id,
            overall_score: db.overall_score,
            as_supplier_score: db.as_supplier_score,
            as_consumer_score: db.as_consumer_score,
            as_enhancer_score: db.as_enhancer_score,
            deals_completed_count: db.deals_completed_count,
            deals_cancelled_count: db.deals_cancelled_count,
            deals_disputed_count: db.deals_disputed_count,
            timeouts_count: db.timeouts_count,
            no_shows_count: db.no_shows_count,
            total_completed_value: db.total_completed_value,
            average_response_hours: db.average_response_hours,
            profile_completeness: db.profile_completeness,
            verification_level: db.verification_level,
            longevity_days: db.longevity_days,
            calculation_formula: db.calculation_formula,
            last_calculated_at: db.last_calculated_at,
            next_calculation_at: db.next_calculation_at,
        }
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
