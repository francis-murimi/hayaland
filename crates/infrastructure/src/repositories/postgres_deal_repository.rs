use async_trait::async_trait;
use domain::entities::{
    Deal, DealParticipation, DealRole, DealStatus, DistributionModel, GeoPoint,
    ParticipationStatus, Term, TermStatus, TermType, ValueDistribution,
};
use domain::errors::DomainError;
use domain::repositories::{DealAggregate, DealListResult, DealRepository, DealSearchCriteria};
use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresDealRepository {
    pool: PgPool,
}

impl PostgresDealRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DealRepository for PostgresDealRepository {
    async fn create(&self, aggregate: &DealAggregate) -> Result<(), DomainError> {
        let deal = &aggregate.deal;
        let (lat, lng) = deal
            .location
            .map(|l| (Some(l.latitude), Some(l.longitude)))
            .unwrap_or((None, None));

        sqlx::query!(
            r#"
            INSERT INTO deals (
                id, deal_reference, deal_title, deal_description, domain_category_id,
                initiator_party_id, initiator_role, deal_status, expected_start_date,
                expected_end_date, actual_start_date, actual_end_date, timeline,
                location_geo, location_address, total_deal_value, currency,
                platform_fee_percentage, platform_fee_amount, win_win_win_validated,
                validation_checked_at, validation_score, validation_result, is_public,
                current_state_entered_at, timeout_overrides, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                CASE
                    WHEN $14::float8 IS NOT NULL AND $15::float8 IS NOT NULL
                    THEN ST_SetSRID(ST_MakePoint($15, $14), 4326)::geography
                    ELSE NULL
                END,
                $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29
            )
            "#,
            deal.id,
            deal.deal_reference,
            deal.deal_title.as_str(),
            deal.deal_description,
            deal.domain_category_id,
            deal.initiator_party_id,
            deal.initiator_role.as_str(),
            deal.deal_status.as_str(),
            deal.expected_start_date,
            deal.expected_end_date,
            deal.actual_start_date,
            deal.actual_end_date,
            deal.timeline,
            lat,
            lng,
            deal.location_address,
            deal.total_deal_value,
            deal.currency,
            deal.platform_fee_percentage,
            deal.platform_fee_amount,
            deal.win_win_win_validated,
            deal.validation_checked_at,
            deal.validation_score,
            deal.validation_result,
            deal.is_public,
            deal.current_state_entered_at,
            deal.timeout_overrides,
            deal.created_at,
            deal.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        for participation in &aggregate.participations {
            sqlx::query!(
                r#"
                INSERT INTO deal_participations (
                    id, deal_id, party_id, role, participation_status, is_initiator,
                    value_share_percentage, value_share_amount, invited_at, responded_at, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
                participation.id,
                participation.deal_id,
                participation.party_id,
                participation.role.as_str(),
                participation.participation_status.as_str(),
                participation.is_initiator,
                participation.value_share_percentage,
                participation.value_share_amount,
                participation.invited_at,
                participation.responded_at,
                participation.created_at
            )
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Deal>, DomainError> {
        let row = sqlx::query_as!(
            DealRow,
            r#"
            SELECT
                id as "id!",
                deal_reference as "deal_reference!",
                deal_title as "deal_title!",
                deal_description,
                domain_category_id as "domain_category_id!",
                initiator_party_id as "initiator_party_id!",
                initiator_role as "initiator_role!",
                deal_status as "deal_status!",
                expected_start_date,
                expected_end_date,
                actual_start_date,
                actual_end_date,
                timeline,
                ST_Y(location_geo::geometry) as latitude,
                ST_X(location_geo::geometry) as longitude,
                location_address,
                total_deal_value,
                currency as "currency!",
                platform_fee_percentage as "platform_fee_percentage!",
                platform_fee_amount as "platform_fee_amount!",
                win_win_win_validated as "win_win_win_validated!",
                validation_checked_at,
                validation_score,
                validation_result,
                is_public as "is_public!",
                current_state_entered_at as "current_state_entered_at!",
                timeout_overrides,
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM deals
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_deal_from_row))
    }

    async fn find_aggregate_by_id(&self, id: Uuid) -> Result<Option<DealAggregate>, DomainError> {
        let deal = match self.find_by_id(id).await? {
            Some(d) => d,
            None => return Ok(None),
        };
        let participations = self.find_participations_by_deal(id).await?;
        Ok(Some(DealAggregate {
            deal,
            participations,
        }))
    }

    async fn find_deals_by_status(
        &self,
        status: DealStatus,
        entered_before: OffsetDateTime,
        limit: i64,
    ) -> Result<Vec<Deal>, DomainError> {
        let rows = sqlx::query_as!(
            DealRow,
            r#"
            SELECT
                id as "id!",
                deal_reference as "deal_reference!",
                deal_title as "deal_title!",
                deal_description,
                domain_category_id as "domain_category_id!",
                initiator_party_id as "initiator_party_id!",
                initiator_role as "initiator_role!",
                deal_status as "deal_status!",
                expected_start_date,
                expected_end_date,
                actual_start_date,
                actual_end_date,
                timeline,
                ST_Y(location_geo::geometry) as latitude,
                ST_X(location_geo::geometry) as longitude,
                location_address,
                total_deal_value,
                currency as "currency!",
                platform_fee_percentage as "platform_fee_percentage!",
                platform_fee_amount as "platform_fee_amount!",
                win_win_win_validated as "win_win_win_validated!",
                validation_checked_at,
                validation_score,
                validation_result,
                is_public as "is_public!",
                current_state_entered_at as "current_state_entered_at!",
                timeout_overrides,
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM deals
            WHERE deal_status = $1
              AND current_state_entered_at < $2
            ORDER BY current_state_entered_at
            LIMIT $3
            FOR UPDATE SKIP LOCKED
            "#,
            status.as_str(),
            entered_before,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_deal_from_row).collect())
    }

    async fn find_participations_by_deal(
        &self,
        deal_id: Uuid,
    ) -> Result<Vec<DealParticipation>, DomainError> {
        let rows = sqlx::query_as!(
            ParticipationRow,
            r#"
            SELECT id, deal_id, party_id, role, participation_status, is_initiator,
                value_share_percentage as "value_share_percentage: _",
                value_share_amount as "value_share_amount: _",
                invited_at, responded_at, created_at
            FROM deal_participations
            WHERE deal_id = $1
            ORDER BY role
            "#,
            deal_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_participation_from_row).collect())
    }

    async fn update(&self, deal: &Deal) -> Result<(), DomainError> {
        let (lat, lng) = deal
            .location
            .map(|l| (Some(l.latitude), Some(l.longitude)))
            .unwrap_or((None, None));

        sqlx::query!(
            r#"
            UPDATE deals
            SET deal_title = $1,
                deal_description = $2,
                domain_category_id = $3,
                initiator_party_id = $4,
                initiator_role = $5,
                deal_status = $6,
                expected_start_date = $7,
                expected_end_date = $8,
                actual_start_date = $9,
                actual_end_date = $10,
                timeline = $11,
                location_geo = CASE
                    WHEN $12::float8 IS NOT NULL AND $13::float8 IS NOT NULL
                    THEN ST_SetSRID(ST_MakePoint($13, $12), 4326)::geography
                    ELSE NULL
                END,
                location_address = $14,
                total_deal_value = $15,
                currency = $16,
                platform_fee_percentage = $17,
                platform_fee_amount = $18,
                win_win_win_validated = $19,
                validation_checked_at = $20,
                validation_score = $21,
                validation_result = $22,
                is_public = $23,
                current_state_entered_at = $24,
                timeout_overrides = $25,
                updated_at = $26
            WHERE id = $27
            "#,
            deal.deal_title.as_str(),
            deal.deal_description,
            deal.domain_category_id,
            deal.initiator_party_id,
            deal.initiator_role.as_str(),
            deal.deal_status.as_str(),
            deal.expected_start_date,
            deal.expected_end_date,
            deal.actual_start_date,
            deal.actual_end_date,
            deal.timeline,
            lat,
            lng,
            deal.location_address,
            deal.total_deal_value,
            deal.currency,
            deal.platform_fee_percentage,
            deal.platform_fee_amount,
            deal.win_win_win_validated,
            deal.validation_checked_at,
            deal.validation_score,
            deal.validation_result,
            deal.is_public,
            deal.current_state_entered_at,
            deal.timeout_overrides,
            deal.updated_at,
            deal.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_participation(
        &self,
        participation: &DealParticipation,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE deal_participations
            SET party_id = $1,
                role = $2,
                participation_status = $3,
                is_initiator = $4,
                value_share_percentage = $5,
                value_share_amount = $6,
                invited_at = $7,
                responded_at = $8
            WHERE id = $9
            "#,
            participation.party_id,
            participation.role.as_str(),
            participation.participation_status.as_str(),
            participation.is_initiator,
            participation.value_share_percentage,
            participation.value_share_amount,
            participation.invited_at,
            participation.responded_at,
            participation.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list(&self, criteria: &DealSearchCriteria) -> Result<DealListResult, DomainError> {
        let statuses: Option<Vec<String>> = criteria.status.map(|s| vec![s.as_str().to_string()]);

        let rows = sqlx::query_as!(
            DealRow,
            r#"
            SELECT
                id as "id!",
                deal_reference as "deal_reference!",
                deal_title as "deal_title!",
                deal_description,
                domain_category_id as "domain_category_id!",
                initiator_party_id as "initiator_party_id!",
                initiator_role as "initiator_role!",
                deal_status as "deal_status!",
                expected_start_date,
                expected_end_date,
                actual_start_date,
                actual_end_date,
                timeline,
                ST_Y(location_geo::geometry) as latitude,
                ST_X(location_geo::geometry) as longitude,
                location_address,
                total_deal_value,
                currency as "currency!",
                platform_fee_percentage as "platform_fee_percentage!",
                platform_fee_amount as "platform_fee_amount!",
                win_win_win_validated as "win_win_win_validated!",
                validation_checked_at,
                validation_score,
                validation_result,
                is_public as "is_public!",
                current_state_entered_at as "current_state_entered_at!",
                timeout_overrides,
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM deals
            WHERE ($1::uuid IS NULL OR EXISTS (
                SELECT 1 FROM deal_participations dp
                WHERE dp.deal_id = deals.id AND dp.party_id = $1
            ))
            AND ($2::text[] IS NULL OR deal_status = ANY($2))
            AND ($3::uuid IS NULL OR initiator_party_id = $3)
            AND ($4::uuid IS NULL OR domain_category_id = $4)
            ORDER BY updated_at DESC
            LIMIT $5 OFFSET $6
            "#,
            criteria.party_id,
            statuses.as_deref(),
            criteria.initiator_party_id,
            criteria.domain_category_id,
            criteria.limit,
            criteria.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM deals
            WHERE ($1::uuid IS NULL OR EXISTS (
                SELECT 1 FROM deal_participations dp
                WHERE dp.deal_id = deals.id AND dp.party_id = $1
            ))
            AND ($2::text[] IS NULL OR deal_status = ANY($2))
            AND ($3::uuid IS NULL OR initiator_party_id = $3)
            AND ($4::uuid IS NULL OR domain_category_id = $4)
            "#,
            criteria.party_id,
            statuses.as_deref(),
            criteria.initiator_party_id,
            criteria.domain_category_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(DealListResult {
            deals: rows.into_iter().map(build_deal_from_row).collect(),
            total,
            limit: criteria.limit,
            offset: criteria.offset,
        })
    }

    async fn count_active_deals_for_party(&self, party_id: Uuid) -> Result<i64, DomainError> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM deal_participations dp
            JOIN deals d ON d.id = dp.deal_id
            WHERE dp.party_id = $1
            AND d.deal_status NOT IN ('COMPLETED', 'CANCELLED', 'EXPIRED')
            "#,
            party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(count)
    }

    async fn count_active_deals_for_party_role(
        &self,
        party_id: Uuid,
        role: DealRole,
    ) -> Result<i64, DomainError> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM deal_participations dp
            JOIN deals d ON d.id = dp.deal_id
            WHERE dp.party_id = $1
            AND dp.role = $2
            AND d.deal_status NOT IN ('COMPLETED', 'CANCELLED', 'EXPIRED')
            "#,
            party_id,
            role.as_str()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(count)
    }

    async fn record_history(
        &self,
        deal_id: Uuid,
        event_type: &str,
        actor_party_id: Option<Uuid>,
        details: Option<serde_json::Value>,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO deal_history (id, deal_id, event_type, actor_party_id, details, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            Uuid::now_v7(),
            deal_id,
            event_type,
            actor_party_id,
            details.unwrap_or(serde_json::Value::Object(Default::default())),
            OffsetDateTime::now_utc()
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn is_party_participant(
        &self,
        deal_id: Uuid,
        party_id: Uuid,
    ) -> Result<bool, DomainError> {
        let exists = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM deal_participations
                WHERE deal_id = $1 AND party_id = $2
            ) as "exists!"
            "#,
            deal_id,
            party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(exists)
    }

    async fn next_deal_reference(&self) -> Result<String, DomainError> {
        let year = OffsetDateTime::now_utc().year();
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!" FROM deals
            WHERE deal_reference LIKE $1
            "#,
            format!("DL-{year}-%")
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(format!("DL-{year}-{:04}", count + 1))
    }

    async fn update_value_totals(
        &self,
        deal_id: Uuid,
        total_value: Decimal,
        platform_fee_percentage: Decimal,
        platform_fee_amount: Decimal,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE deals
            SET total_deal_value = $1,
                platform_fee_percentage = $2,
                platform_fee_amount = $3,
                updated_at = $4
            WHERE id = $5
            "#,
            total_value,
            platform_fee_percentage,
            platform_fee_amount,
            OffsetDateTime::now_utc(),
            deal_id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn create_term(&self, term: &Term) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO terms (
                id, deal_id, proposed_by_party_id, term_type, term_name, description,
                negotiation_status, parent_term_id, version, proposed_at, resolved_at,
                is_mandatory, resolution, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
            term.id,
            term.deal_id,
            term.proposed_by_party_id,
            term.term_type.as_str(),
            term.term_name,
            term.description,
            term.negotiation_status.as_str(),
            term.parent_term_id,
            term.version,
            term.proposed_at,
            term.resolved_at,
            term.is_mandatory,
            term.resolution,
            term.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_term(&self, term: &Term) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE terms
            SET term_type = $1,
                term_name = $2,
                description = $3,
                negotiation_status = $4,
                parent_term_id = $5,
                version = $6,
                proposed_at = $7,
                resolved_at = $8,
                is_mandatory = $9,
                resolution = $10
            WHERE id = $11
            "#,
            term.term_type.as_str(),
            term.term_name,
            term.description,
            term.negotiation_status.as_str(),
            term.parent_term_id,
            term.version,
            term.proposed_at,
            term.resolved_at,
            term.is_mandatory,
            term.resolution,
            term.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_term_by_id(&self, id: Uuid) -> Result<Option<Term>, DomainError> {
        let row = sqlx::query_as!(
            TermRow,
            r#"
            SELECT id, deal_id, proposed_by_party_id, term_type, term_name, description,
                negotiation_status, parent_term_id, version, proposed_at, resolved_at,
                is_mandatory, resolution, created_at
            FROM terms
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_term_from_row))
    }

    async fn find_terms_by_deal(&self, deal_id: Uuid) -> Result<Vec<Term>, DomainError> {
        let rows = sqlx::query_as!(
            TermRow,
            r#"
            SELECT id, deal_id, proposed_by_party_id, term_type, term_name, description,
                negotiation_status, parent_term_id, version, proposed_at, resolved_at,
                is_mandatory, resolution, created_at
            FROM terms
            WHERE deal_id = $1
            ORDER BY term_type, version
            "#,
            deal_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_term_from_row).collect())
    }

    async fn set_value_distribution(
        &self,
        distribution: &ValueDistribution,
    ) -> Result<(), DomainError> {
        let payment_schedule =
            serde_json::to_value(&distribution.payment_schedule).map_err(|e| {
                DomainError::InvalidValueDistribution {
                    message: e.to_string(),
                }
            })?;

        sqlx::query!(
            r#"
            INSERT INTO value_distributions (
                id, deal_id, total_value, currency, distribution_model,
                supplier_share_percentage, supplier_share_amount,
                consumer_cost_percentage, consumer_cost_amount,
                enhancer_share_percentage, enhancer_share_amount,
                platform_fee_percentage, platform_fee_amount,
                payment_schedule, win_win_win_score, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            ON CONFLICT (deal_id) DO UPDATE SET
                total_value = EXCLUDED.total_value,
                currency = EXCLUDED.currency,
                distribution_model = EXCLUDED.distribution_model,
                supplier_share_percentage = EXCLUDED.supplier_share_percentage,
                supplier_share_amount = EXCLUDED.supplier_share_amount,
                consumer_cost_percentage = EXCLUDED.consumer_cost_percentage,
                consumer_cost_amount = EXCLUDED.consumer_cost_amount,
                enhancer_share_percentage = EXCLUDED.enhancer_share_percentage,
                enhancer_share_amount = EXCLUDED.enhancer_share_amount,
                platform_fee_percentage = EXCLUDED.platform_fee_percentage,
                platform_fee_amount = EXCLUDED.platform_fee_amount,
                payment_schedule = EXCLUDED.payment_schedule,
                win_win_win_score = EXCLUDED.win_win_win_score,
                updated_at = EXCLUDED.updated_at
            "#,
            distribution.id,
            distribution.deal_id,
            distribution.total_value,
            distribution.currency,
            distribution.distribution_model.as_str(),
            distribution.supplier_share_percentage,
            distribution.supplier_share_amount,
            distribution.consumer_cost_percentage,
            distribution.consumer_cost_amount,
            distribution.enhancer_share_percentage,
            distribution.enhancer_share_amount,
            distribution.platform_fee_percentage,
            distribution.platform_fee_amount,
            payment_schedule,
            distribution.win_win_win_score,
            distribution.created_at,
            distribution.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_value_distribution_by_deal(
        &self,
        deal_id: Uuid,
    ) -> Result<Option<ValueDistribution>, DomainError> {
        let row = sqlx::query_as!(
            ValueDistributionRow,
            r#"
            SELECT id, deal_id, total_value, currency, distribution_model,
                supplier_share_percentage, supplier_share_amount,
                consumer_cost_percentage, consumer_cost_amount,
                enhancer_share_percentage, enhancer_share_amount,
                platform_fee_percentage, platform_fee_amount,
                payment_schedule as "payment_schedule!: serde_json::Value",
                win_win_win_score, created_at, updated_at
            FROM value_distributions
            WHERE deal_id = $1
            "#,
            deal_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_value_distribution_from_row))
    }
}

#[derive(sqlx::FromRow)]
struct DealRow {
    id: Uuid,
    deal_reference: String,
    deal_title: String,
    deal_description: Option<String>,
    domain_category_id: Uuid,
    initiator_party_id: Uuid,
    initiator_role: String,
    deal_status: String,
    expected_start_date: Option<time::Date>,
    expected_end_date: Option<time::Date>,
    actual_start_date: Option<time::Date>,
    actual_end_date: Option<time::Date>,
    timeline: Option<serde_json::Value>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    location_address: Option<serde_json::Value>,
    total_deal_value: Option<Decimal>,
    currency: String,
    platform_fee_percentage: Decimal,
    platform_fee_amount: Decimal,
    win_win_win_validated: bool,
    validation_checked_at: Option<OffsetDateTime>,
    validation_score: Option<Decimal>,
    validation_result: Option<serde_json::Value>,
    is_public: bool,
    current_state_entered_at: OffsetDateTime,
    timeout_overrides: Option<serde_json::Value>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct ParticipationRow {
    id: Uuid,
    deal_id: Uuid,
    party_id: Uuid,
    role: String,
    participation_status: String,
    is_initiator: bool,
    value_share_percentage: Option<rust_decimal::Decimal>,
    value_share_amount: Option<rust_decimal::Decimal>,
    invited_at: Option<OffsetDateTime>,
    responded_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct TermRow {
    id: Uuid,
    deal_id: Uuid,
    proposed_by_party_id: Uuid,
    term_type: String,
    term_name: String,
    description: String,
    negotiation_status: String,
    parent_term_id: Option<Uuid>,
    version: i32,
    proposed_at: OffsetDateTime,
    resolved_at: Option<OffsetDateTime>,
    is_mandatory: bool,
    resolution: Option<String>,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct ValueDistributionRow {
    id: Uuid,
    deal_id: Uuid,
    total_value: Decimal,
    currency: String,
    distribution_model: String,
    supplier_share_percentage: Decimal,
    supplier_share_amount: Decimal,
    consumer_cost_percentage: Decimal,
    consumer_cost_amount: Decimal,
    enhancer_share_percentage: Decimal,
    enhancer_share_amount: Decimal,
    platform_fee_percentage: Decimal,
    platform_fee_amount: Decimal,
    payment_schedule: serde_json::Value,
    win_win_win_score: Option<Decimal>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn build_deal_from_row(row: DealRow) -> Deal {
    Deal {
        id: row.id,
        deal_reference: row.deal_reference,
        deal_title: domain::entities::DealTitle::new(&row.deal_title)
            .expect("database should contain valid deal titles"),
        deal_description: row.deal_description,
        domain_category_id: row.domain_category_id,
        initiator_party_id: row.initiator_party_id,
        initiator_role: DealRole::try_from(row.initiator_role.as_str())
            .expect("database should contain valid deal roles"),
        deal_status: DealStatus::try_from(row.deal_status.as_str())
            .expect("database should contain valid deal statuses"),
        expected_start_date: row.expected_start_date,
        expected_end_date: row.expected_end_date,
        actual_start_date: row.actual_start_date,
        actual_end_date: row.actual_end_date,
        timeline: row.timeline,
        location: match (row.latitude, row.longitude) {
            (Some(lat), Some(lng)) => GeoPoint::new(lat, lng).ok(),
            _ => None,
        },
        location_address: row.location_address,
        total_deal_value: row.total_deal_value,
        currency: row.currency,
        platform_fee_percentage: row.platform_fee_percentage,
        platform_fee_amount: row.platform_fee_amount,
        win_win_win_validated: row.win_win_win_validated,
        validation_checked_at: row.validation_checked_at,
        validation_score: row.validation_score,
        validation_result: row.validation_result,
        is_public: row.is_public,
        current_state_entered_at: row.current_state_entered_at,
        timeout_overrides: row.timeout_overrides,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn build_participation_from_row(row: ParticipationRow) -> DealParticipation {
    DealParticipation {
        id: row.id,
        deal_id: row.deal_id,
        party_id: row.party_id,
        role: DealRole::try_from(row.role.as_str())
            .expect("database should contain valid deal roles"),
        participation_status: ParticipationStatus::try_from(row.participation_status.as_str())
            .expect("database should contain valid participation statuses"),
        is_initiator: row.is_initiator,
        value_share_percentage: row.value_share_percentage,
        value_share_amount: row.value_share_amount,
        invited_at: row.invited_at,
        responded_at: row.responded_at,
        created_at: row.created_at,
    }
}

fn build_term_from_row(row: TermRow) -> Term {
    Term {
        id: row.id,
        deal_id: row.deal_id,
        proposed_by_party_id: row.proposed_by_party_id,
        term_type: TermType::try_from(row.term_type.as_str())
            .expect("database should contain valid term types"),
        term_name: row.term_name,
        description: row.description,
        negotiation_status: TermStatus::try_from(row.negotiation_status.as_str())
            .expect("database should contain valid term statuses"),
        parent_term_id: row.parent_term_id,
        version: row.version,
        proposed_at: row.proposed_at,
        resolved_at: row.resolved_at,
        is_mandatory: row.is_mandatory,
        resolution: row.resolution,
        created_at: row.created_at,
    }
}

fn build_value_distribution_from_row(row: ValueDistributionRow) -> ValueDistribution {
    let payment_schedule: Vec<domain::entities::PaymentScheduleEntry> =
        serde_json::from_value(row.payment_schedule)
            .expect("database should contain valid payment schedule JSON");

    ValueDistribution {
        id: row.id,
        deal_id: row.deal_id,
        total_value: row.total_value,
        currency: row.currency,
        distribution_model: DistributionModel::try_from(row.distribution_model.as_str())
            .expect("database should contain valid distribution models"),
        supplier_share_percentage: row.supplier_share_percentage,
        supplier_share_amount: row.supplier_share_amount,
        consumer_cost_percentage: row.consumer_cost_percentage,
        consumer_cost_amount: row.consumer_cost_amount,
        enhancer_share_percentage: row.enhancer_share_percentage,
        enhancer_share_amount: row.enhancer_share_amount,
        platform_fee_percentage: row.platform_fee_percentage,
        platform_fee_amount: row.platform_fee_amount,
        payment_schedule,
        win_win_win_score: row.win_win_win_score,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
