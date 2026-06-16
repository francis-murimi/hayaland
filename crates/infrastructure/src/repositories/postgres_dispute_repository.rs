use async_trait::async_trait;
use domain::entities::{
    Dispute, DisputeResponse, DisputeSeverity, DisputeStatus, DisputeType, ResolutionOutcome,
    ResolutionType,
};
use domain::errors::DomainError;
use domain::repositories::{DisputeFilters, DisputeListResult, DisputeRepository};
use sqlx::{Error as SqlxError, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresDisputeRepository {
    pool: PgPool,
}

impl PostgresDisputeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DisputeRepository for PostgresDisputeRepository {
    async fn create(&self, dispute: &Dispute) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO disputes (
                id, deal_id, raised_by_party_id, raised_by_user_id, against_party_id,
                dispute_type, dispute_status, resolution_type, resolution_outcome, severity,
                description, evidence_urls, admin_notes, resolution_notes, resolved_by_user_id,
                resolved_at, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18
            )
            "#,
            dispute.id,
            dispute.deal_id,
            dispute.raised_by_party_id,
            dispute.raised_by_user_id,
            dispute.against_party_id,
            dispute.dispute_type.as_str(),
            dispute.dispute_status.as_str(),
            dispute.resolution_type.map(|r| r.as_str()),
            dispute.resolution_outcome.map(|r| r.as_str()),
            dispute.severity.map(|s| s.as_str()),
            dispute.description,
            &dispute.evidence_urls,
            dispute.admin_notes,
            dispute.resolution_notes,
            dispute.resolved_by_user_id,
            dispute.resolved_at,
            dispute.created_at,
            dispute.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Dispute>, DomainError> {
        let row = sqlx::query_as!(
            DisputeRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                raised_by_party_id as "raised_by_party_id!",
                raised_by_user_id as "raised_by_user_id!",
                against_party_id,
                dispute_type as "dispute_type!",
                dispute_status as "dispute_status!",
                resolution_type,
                resolution_outcome,
                severity,
                description as "description!",
                evidence_urls as "evidence_urls!",
                admin_notes,
                resolution_notes,
                resolved_by_user_id,
                resolved_at,
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM disputes
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_dispute_from_row))
    }

    async fn list_by_deal(
        &self,
        deal_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<DisputeListResult, DomainError> {
        let disputes = sqlx::query_as!(
            DisputeRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                raised_by_party_id as "raised_by_party_id!",
                raised_by_user_id as "raised_by_user_id!",
                against_party_id,
                dispute_type as "dispute_type!",
                dispute_status as "dispute_status!",
                resolution_type,
                resolution_outcome,
                severity,
                description as "description!",
                evidence_urls as "evidence_urls!",
                admin_notes,
                resolution_notes,
                resolved_by_user_id,
                resolved_at,
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM disputes
            WHERE deal_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            OFFSET $3
            "#,
            deal_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = self.count_by_deal(deal_id).await?;

        Ok(DisputeListResult {
            disputes: disputes.into_iter().map(build_dispute_from_row).collect(),
            total,
            limit,
            offset,
        })
    }

    async fn list_admin(&self, filters: &DisputeFilters) -> Result<DisputeListResult, DomainError> {
        let disputes = sqlx::query_as!(
            DisputeRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                raised_by_party_id as "raised_by_party_id!",
                raised_by_user_id as "raised_by_user_id!",
                against_party_id,
                dispute_type as "dispute_type!",
                dispute_status as "dispute_status!",
                resolution_type,
                resolution_outcome,
                severity,
                description as "description!",
                evidence_urls as "evidence_urls!",
                admin_notes,
                resolution_notes,
                resolved_by_user_id,
                resolved_at,
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM disputes
            WHERE ($1::text IS NULL OR dispute_status = $1)
              AND ($2::uuid IS NULL OR deal_id = $2)
              AND ($3::uuid IS NULL OR raised_by_party_id = $3)
              AND ($4::uuid IS NULL OR against_party_id = $4)
            ORDER BY created_at DESC
            LIMIT $5
            OFFSET $6
            "#,
            filters.status.map(|s| s.as_str()),
            filters.deal_id,
            filters.raised_by_party_id,
            filters.against_party_id,
            filters.limit,
            filters.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = self.count_admin(filters).await?;

        Ok(DisputeListResult {
            disputes: disputes.into_iter().map(build_dispute_from_row).collect(),
            total,
            limit: filters.limit,
            offset: filters.offset,
        })
    }

    async fn submit_evidence(
        &self,
        id: Uuid,
        evidence_urls: Vec<String>,
        notes: Option<String>,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE disputes
            SET evidence_urls = evidence_urls || $1,
                admin_notes = COALESCE($2, admin_notes),
                dispute_status = CASE WHEN dispute_status = 'OPEN' THEN 'UNDER_REVIEW' ELSE dispute_status END,
                updated_at = now()
            WHERE id = $3
            "#,
            &evidence_urls,
            notes,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn add_response(&self, response: &DisputeResponse) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO dispute_responses (id, dispute_id, party_id, user_id, message, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            response.id,
            response.dispute_id,
            response.party_id,
            response.user_id,
            response.message,
            response.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        // Optionally move dispute to UNDER_REVIEW if still OPEN.
        sqlx::query!(
            r#"
            UPDATE disputes
            SET dispute_status = CASE WHEN dispute_status = 'OPEN' THEN 'UNDER_REVIEW' ELSE dispute_status END,
                updated_at = now()
            WHERE id = $1
            "#,
            response.dispute_id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list_responses(&self, dispute_id: Uuid) -> Result<Vec<DisputeResponse>, DomainError> {
        let rows = sqlx::query_as!(
            DisputeResponseRow,
            r#"
            SELECT
                id as "id!",
                dispute_id as "dispute_id!",
                party_id as "party_id!",
                user_id as "user_id!",
                message as "message!",
                created_at as "created_at!"
            FROM dispute_responses
            WHERE dispute_id = $1
            ORDER BY created_at ASC
            "#,
            dispute_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows
            .into_iter()
            .map(|r| DisputeResponse {
                id: r.id,
                dispute_id: r.dispute_id,
                party_id: r.party_id,
                user_id: r.user_id,
                message: r.message,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn escalate(
        &self,
        id: Uuid,
        escalated_by_user_id: Uuid,
        notes: Option<String>,
    ) -> Result<(), DomainError> {
        let rows = sqlx::query!(
            r#"
            UPDATE disputes
            SET dispute_status = 'ESCALATED',
                admin_notes = COALESCE($1, admin_notes),
                resolved_by_user_id = $2,
                updated_at = now()
            WHERE id = $3
              AND dispute_status NOT IN ('RESOLVED', 'REJECTED')
            "#,
            notes,
            escalated_by_user_id,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if rows.rows_affected() == 0 {
            return Err(DomainError::DisputeNotFound);
        }

        Ok(())
    }

    async fn resolve(
        &self,
        id: Uuid,
        resolved_by_user_id: Uuid,
        resolution_type: ResolutionType,
        resolution_outcome: ResolutionOutcome,
        severity: DisputeSeverity,
        resolution_notes: Option<String>,
    ) -> Result<(), DomainError> {
        let rows = sqlx::query!(
            r#"
            UPDATE disputes
            SET dispute_status = 'RESOLVED',
                resolution_type = $1,
                resolution_outcome = $2,
                severity = $3,
                resolution_notes = $4,
                resolved_by_user_id = $5,
                resolved_at = now(),
                updated_at = now()
            WHERE id = $6
              AND dispute_status NOT IN ('RESOLVED', 'REJECTED')
            "#,
            resolution_type.as_str(),
            resolution_outcome.as_str(),
            severity.as_str(),
            resolution_notes,
            resolved_by_user_id,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if rows.rows_affected() == 0 {
            return Err(DomainError::DisputeNotFound);
        }

        Ok(())
    }

    async fn reject(
        &self,
        id: Uuid,
        resolved_by_user_id: Uuid,
        reason: String,
    ) -> Result<(), DomainError> {
        let rows = sqlx::query!(
            r#"
            UPDATE disputes
            SET dispute_status = 'REJECTED',
                resolution_notes = $1,
                resolved_by_user_id = $2,
                resolved_at = now(),
                updated_at = now()
            WHERE id = $3
              AND dispute_status NOT IN ('RESOLVED', 'REJECTED')
            "#,
            reason,
            resolved_by_user_id,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if rows.rows_affected() == 0 {
            return Err(DomainError::DisputeNotFound);
        }

        Ok(())
    }

    async fn count_open_by_party(&self, party_id: Uuid) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM disputes
            WHERE raised_by_party_id = $1
              AND dispute_status NOT IN ('RESOLVED', 'REJECTED')
            "#,
            party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn count_open_against_party(&self, party_id: Uuid) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM disputes
            WHERE against_party_id = $1
              AND dispute_status NOT IN ('RESOLVED', 'REJECTED')
            "#,
            party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn increment_deals_disputed_count(&self, party_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO trust_scores (id, party_id, deals_completed_count, deals_cancelled_count, deals_disputed_count)
            VALUES ($1, $2, 0, 0, 1)
            ON CONFLICT (party_id) DO UPDATE SET
                deals_disputed_count = trust_scores.deals_disputed_count + 1,
                updated_at = now()
            "#,
            Uuid::now_v7(),
            party_id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: DisputeStatus,
        updated_at: OffsetDateTime,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE disputes
            SET dispute_status = $1,
                updated_at = $2
            WHERE id = $3
            "#,
            status.as_str(),
            updated_at,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }
}

impl PostgresDisputeRepository {
    async fn count_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM disputes
            WHERE deal_id = $1
            "#,
            deal_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn count_admin(&self, filters: &DisputeFilters) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM disputes
            WHERE ($1::text IS NULL OR dispute_status = $1)
              AND ($2::uuid IS NULL OR deal_id = $2)
              AND ($3::uuid IS NULL OR raised_by_party_id = $3)
              AND ($4::uuid IS NULL OR against_party_id = $4)
            "#,
            filters.status.map(|s| s.as_str()),
            filters.deal_id,
            filters.raised_by_party_id,
            filters.against_party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }
}

#[derive(sqlx::FromRow)]
struct DisputeRow {
    id: Uuid,
    deal_id: Uuid,
    raised_by_party_id: Uuid,
    raised_by_user_id: Uuid,
    against_party_id: Option<Uuid>,
    dispute_type: String,
    dispute_status: String,
    resolution_type: Option<String>,
    resolution_outcome: Option<String>,
    severity: Option<String>,
    description: String,
    evidence_urls: Vec<String>,
    admin_notes: Option<String>,
    resolution_notes: Option<String>,
    resolved_by_user_id: Option<Uuid>,
    resolved_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct DisputeResponseRow {
    id: Uuid,
    dispute_id: Uuid,
    party_id: Uuid,
    user_id: Uuid,
    message: String,
    created_at: OffsetDateTime,
}

fn build_dispute_from_row(row: DisputeRow) -> Dispute {
    Dispute {
        id: row.id,
        deal_id: row.deal_id,
        raised_by_party_id: row.raised_by_party_id,
        raised_by_user_id: row.raised_by_user_id,
        against_party_id: row.against_party_id,
        dispute_type: DisputeType::try_from(row.dispute_type.as_str())
            .expect("database contains valid dispute types"),
        dispute_status: DisputeStatus::try_from(row.dispute_status.as_str())
            .expect("database contains valid dispute statuses"),
        resolution_type: row
            .resolution_type
            .and_then(|s| ResolutionType::try_from(s.as_str()).ok()),
        resolution_outcome: row
            .resolution_outcome
            .and_then(|s| ResolutionOutcome::try_from(s.as_str()).ok()),
        severity: row
            .severity
            .and_then(|s| DisputeSeverity::try_from(s.as_str()).ok()),
        description: row.description,
        evidence_urls: row.evidence_urls,
        admin_notes: row.admin_notes,
        resolution_notes: row.resolution_notes,
        resolved_by_user_id: row.resolved_by_user_id,
        resolved_at: row.resolved_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    if let SqlxError::Database(db_err) = &err {
        if db_err.constraint() == Some("idx_disputes_unique_open") {
            return DomainError::DisputeAlreadyExists;
        }
    }
    DomainError::RepositoryError(err.to_string())
}
