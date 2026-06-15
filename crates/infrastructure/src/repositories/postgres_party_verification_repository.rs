use async_trait::async_trait;
use domain::entities::{PartyVerification, PartyVerificationStatus, PartyVerificationType};
use domain::errors::DomainError;
use domain::repositories::{
    PartyVerificationRepository, VerificationListFilters, VerificationListResult,
};
use serde_json::Value;
use sqlx::{Error as SqlxError, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresPartyVerificationRepository {
    pool: PgPool,
}

impl PostgresPartyVerificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PartyVerificationRepository for PostgresPartyVerificationRepository {
    async fn create(&self, verification: &PartyVerification) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO party_verifications (
                id, party_id, requested_by_user_id, reviewed_by_user_id, verification_type,
                status, points, evidence_urls, provider_reference, provider_payload,
                rejection_reason, review_notes, requested_at, reviewed_at, expires_at,
                created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17
            )
            "#,
            verification.id,
            verification.party_id,
            verification.requested_by_user_id,
            verification.reviewed_by_user_id,
            verification.verification_type.as_str(),
            verification.status.as_str(),
            verification.points,
            &verification.evidence_urls,
            verification.provider_reference,
            verification.provider_payload,
            verification.rejection_reason,
            verification.review_notes,
            verification.requested_at,
            verification.reviewed_at,
            verification.expires_at,
            verification.created_at,
            verification.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PartyVerification>, DomainError> {
        let row = sqlx::query_as!(
            PartyVerificationRow,
            r#"
            SELECT
                id as "id!",
                party_id as "party_id!",
                requested_by_user_id as "requested_by_user_id!",
                reviewed_by_user_id,
                verification_type as "verification_type!",
                status as "status!",
                points as "points!",
                evidence_urls as "evidence_urls!",
                provider_reference,
                provider_payload,
                rejection_reason,
                review_notes,
                requested_at as "requested_at!",
                reviewed_at,
                expires_at,
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM party_verifications
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_verification_from_row))
    }

    async fn find_active_by_party_and_type(
        &self,
        party_id: Uuid,
        verification_type: PartyVerificationType,
    ) -> Result<Option<PartyVerification>, DomainError> {
        let row = sqlx::query_as!(
            PartyVerificationRow,
            r#"
            SELECT
                id as "id!",
                party_id as "party_id!",
                requested_by_user_id as "requested_by_user_id!",
                reviewed_by_user_id,
                verification_type as "verification_type!",
                status as "status!",
                points as "points!",
                evidence_urls as "evidence_urls!",
                provider_reference,
                provider_payload,
                rejection_reason,
                review_notes,
                requested_at as "requested_at!",
                reviewed_at,
                expires_at,
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM party_verifications
            WHERE party_id = $1
              AND verification_type = $2
              AND status IN ('PENDING', 'APPROVED')
            "#,
            party_id,
            verification_type.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_verification_from_row))
    }

    async fn list_by_party(&self, party_id: Uuid) -> Result<Vec<PartyVerification>, DomainError> {
        let rows = sqlx::query_as!(
            PartyVerificationRow,
            r#"
            SELECT
                id as "id!",
                party_id as "party_id!",
                requested_by_user_id as "requested_by_user_id!",
                reviewed_by_user_id,
                verification_type as "verification_type!",
                status as "status!",
                points as "points!",
                evidence_urls as "evidence_urls!",
                provider_reference,
                provider_payload,
                rejection_reason,
                review_notes,
                requested_at as "requested_at!",
                reviewed_at,
                expires_at,
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM party_verifications
            WHERE party_id = $1
            ORDER BY requested_at DESC
            "#,
            party_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_verification_from_row).collect())
    }

    async fn list(
        &self,
        filters: &VerificationListFilters,
    ) -> Result<VerificationListResult, DomainError> {
        let rows = sqlx::query_as!(
            PartyVerificationRow,
            r#"
            SELECT
                id as "id!",
                party_id as "party_id!",
                requested_by_user_id as "requested_by_user_id!",
                reviewed_by_user_id,
                verification_type as "verification_type!",
                status as "status!",
                points as "points!",
                evidence_urls as "evidence_urls!",
                provider_reference,
                provider_payload,
                rejection_reason,
                review_notes,
                requested_at as "requested_at!",
                reviewed_at,
                expires_at,
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM party_verifications
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::text IS NULL OR verification_type = $2)
              AND ($3::uuid IS NULL OR party_id = $3)
            ORDER BY requested_at DESC
            LIMIT $4 OFFSET $5
            "#,
            filters.status,
            filters.verification_type,
            filters.party_id,
            filters.limit,
            filters.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = self.count(filters).await?;

        Ok(VerificationListResult {
            verifications: rows.into_iter().map(build_verification_from_row).collect(),
            total,
            limit: filters.limit,
            offset: filters.offset,
        })
    }

    async fn count(&self, filters: &VerificationListFilters) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM party_verifications
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::text IS NULL OR verification_type = $2)
              AND ($3::uuid IS NULL OR party_id = $3)
            "#,
            filters.status,
            filters.verification_type,
            filters.party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn approve(
        &self,
        id: Uuid,
        reviewed_by_user_id: Uuid,
        review_notes: Option<String>,
    ) -> Result<(), DomainError> {
        let now = OffsetDateTime::now_utc();
        let result = sqlx::query!(
            r#"
            UPDATE party_verifications
            SET status = 'APPROVED',
                reviewed_by_user_id = $2,
                reviewed_at = $3,
                review_notes = COALESCE($4, review_notes),
                updated_at = $3
            WHERE id = $1 AND status = 'PENDING'
            "#,
            id,
            reviewed_by_user_id,
            now,
            review_notes
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::InvalidVerificationStateTransition {
                from: "non-pending".to_string(),
                to: "APPROVED".to_string(),
            });
        }

        Ok(())
    }

    async fn reject(
        &self,
        id: Uuid,
        reviewed_by_user_id: Uuid,
        rejection_reason: String,
        review_notes: Option<String>,
    ) -> Result<(), DomainError> {
        let now = OffsetDateTime::now_utc();
        let result = sqlx::query!(
            r#"
            UPDATE party_verifications
            SET status = 'REJECTED',
                reviewed_by_user_id = $2,
                reviewed_at = $3,
                rejection_reason = $4,
                review_notes = COALESCE($5, review_notes),
                updated_at = $3
            WHERE id = $1 AND status = 'PENDING'
            "#,
            id,
            reviewed_by_user_id,
            now,
            rejection_reason,
            review_notes
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::InvalidVerificationStateTransition {
                from: "non-pending".to_string(),
                to: "REJECTED".to_string(),
            });
        }

        Ok(())
    }

    async fn revoke(
        &self,
        id: Uuid,
        reviewed_by_user_id: Uuid,
        reason: String,
        review_notes: Option<String>,
    ) -> Result<(), DomainError> {
        let now = OffsetDateTime::now_utc();
        let result = sqlx::query!(
            r#"
            UPDATE party_verifications
            SET status = 'REVOKED',
                reviewed_by_user_id = $2,
                reviewed_at = $3,
                rejection_reason = $4,
                review_notes = COALESCE($5, review_notes),
                updated_at = $3
            WHERE id = $1 AND status = 'APPROVED'
            "#,
            id,
            reviewed_by_user_id,
            now,
            reason,
            review_notes
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::InvalidVerificationStateTransition {
                from: "non-approved".to_string(),
                to: "REVOKED".to_string(),
            });
        }

        Ok(())
    }

    async fn sum_approved_points(&self, party_id: Uuid) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COALESCE(SUM(points), 0) as "sum!"
            FROM party_verifications
            WHERE party_id = $1
              AND status = 'APPROVED'
              AND (expires_at IS NULL OR expires_at > now())
            "#,
            party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn count_by_status(&self, party_id: Uuid, status: &str) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM party_verifications
            WHERE party_id = $1 AND status = $2
            "#,
            party_id,
            status
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn set_provider_reference(
        &self,
        id: Uuid,
        provider_reference: String,
        provider_payload: Option<Value>,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE party_verifications
            SET provider_reference = $2,
                provider_payload = $3,
                updated_at = now()
            WHERE id = $1
            "#,
            id,
            provider_reference,
            provider_payload
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn mark_expired(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE party_verifications
            SET status = 'EXPIRED',
                updated_at = now()
            WHERE id = $1 AND status = 'APPROVED'
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_verification_level(
        &self,
        party_id: Uuid,
        verification_level: i32,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO trust_scores (
                id, party_id, overall_score, deals_completed_count, deals_cancelled_count,
                deals_disputed_count, profile_completeness, verification_level, longevity_days,
                calculation_formula, created_at, updated_at
            )
            VALUES (
                $1, $2, 0, 0, 0, 0, 0, $3, 0, '{}', now(), now()
            )
            ON CONFLICT (party_id) DO UPDATE SET
                verification_level = EXCLUDED.verification_level,
                updated_at = now()
            "#,
            Uuid::now_v7(),
            party_id,
            verification_level
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct PartyVerificationRow {
    id: Uuid,
    party_id: Uuid,
    requested_by_user_id: Uuid,
    reviewed_by_user_id: Option<Uuid>,
    verification_type: String,
    status: String,
    points: i32,
    evidence_urls: Vec<String>,
    provider_reference: Option<String>,
    provider_payload: Option<Value>,
    rejection_reason: Option<String>,
    review_notes: Option<String>,
    requested_at: OffsetDateTime,
    reviewed_at: Option<OffsetDateTime>,
    expires_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn build_verification_from_row(row: PartyVerificationRow) -> PartyVerification {
    PartyVerification {
        id: row.id,
        party_id: row.party_id,
        requested_by_user_id: row.requested_by_user_id,
        reviewed_by_user_id: row.reviewed_by_user_id,
        verification_type: PartyVerificationType::try_from(row.verification_type.as_str())
            .expect("database contains valid verification types"),
        status: PartyVerificationStatus::try_from(row.status.as_str())
            .expect("database contains valid verification statuses"),
        points: row.points,
        evidence_urls: row.evidence_urls,
        provider_reference: row.provider_reference,
        provider_payload: row.provider_payload,
        rejection_reason: row.rejection_reason,
        review_notes: row.review_notes,
        requested_at: row.requested_at,
        reviewed_at: row.reviewed_at,
        expires_at: row.expires_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    if let SqlxError::Database(db_err) = &err {
        if db_err.constraint() == Some("idx_party_verifications_unique_active") {
            return DomainError::DuplicateVerification;
        }
    }
    DomainError::RepositoryError(err.to_string())
}

#[cfg(test)]
mod tests {
    use domain::entities::verification_level_from_points;

    #[test]
    fn verification_level_from_points_matches_spec() {
        assert_eq!(verification_level_from_points(0), 0);
        assert_eq!(verification_level_from_points(10), 1);
        assert_eq!(verification_level_from_points(25), 2);
        assert_eq!(verification_level_from_points(55), 3);
        assert_eq!(verification_level_from_points(80), 4);
        assert_eq!(verification_level_from_points(100), 5);
    }
}
