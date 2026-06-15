use async_trait::async_trait;
use domain::entities::{DealRole, Review, ReviewRating};
use domain::errors::DomainError;
use domain::repositories::{ReviewListResult, ReviewRepository, ReviewSearchCriteria};
use sqlx::{Error as SqlxError, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresReviewRepository {
    pool: PgPool,
}

impl PostgresReviewRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReviewRepository for PostgresReviewRepository {
    async fn create(&self, review: &Review) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO reviews (
                id, deal_id, reviewer_party_id, reviewed_party_id, reviewed_role,
                overall_rating, communication_rating, reliability_rating, quality_rating,
                timeliness_rating, review_text, is_verified, is_public, platform_response, created_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
            )
            "#,
            review.id,
            review.deal_id,
            review.reviewer_party_id,
            review.reviewed_party_id,
            review.reviewed_role.as_str(),
            review.overall_rating.value() as i32,
            review.communication_rating.map(|r| r.value() as i32),
            review.reliability_rating.map(|r| r.value() as i32),
            review.quality_rating.map(|r| r.value() as i32),
            review.timeliness_rating.map(|r| r.value() as i32),
            review.review_text,
            review.is_verified,
            review.is_public,
            review.platform_response,
            review.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Review>, DomainError> {
        let row = sqlx::query_as!(
            ReviewRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                reviewer_party_id as "reviewer_party_id!",
                reviewed_party_id as "reviewed_party_id!",
                reviewed_role as "reviewed_role!",
                overall_rating as "overall_rating!",
                communication_rating as "communication_rating: _",
                reliability_rating as "reliability_rating: _",
                quality_rating as "quality_rating: _",
                timeliness_rating as "timeliness_rating: _",
                review_text,
                is_verified as "is_verified!",
                is_public as "is_public!",
                platform_response,
                created_at as "created_at!"
            FROM reviews
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_review_from_row))
    }

    async fn exists(
        &self,
        deal_id: Uuid,
        reviewer_party_id: Uuid,
        reviewed_party_id: Uuid,
    ) -> Result<bool, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM reviews
                WHERE deal_id = $1
                  AND reviewer_party_id = $2
                  AND reviewed_party_id = $3
            ) as "exists!"
            "#,
            deal_id,
            reviewer_party_id,
            reviewed_party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn count_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM reviews
            WHERE deal_id = $1
            "#,
            deal_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn find_missing_review_pairs(
        &self,
        deal_id: Uuid,
        participations: &[(Uuid, DealRole)],
    ) -> Result<Vec<(Uuid, Uuid)>, DomainError> {
        let party_ids: Vec<Uuid> = participations.iter().map(|(id, _)| *id).collect();

        let rows = sqlx::query_as!(
            ExistingReviewPair,
            r#"
            SELECT reviewer_party_id as "reviewer_party_id!", reviewed_party_id as "reviewed_party_id!"
            FROM reviews
            WHERE deal_id = $1
              AND reviewer_party_id = ANY($2)
              AND reviewed_party_id = ANY($2)
            "#,
            deal_id,
            &party_ids
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let existing: std::collections::HashSet<(Uuid, Uuid)> = rows
            .into_iter()
            .map(|r| (r.reviewer_party_id, r.reviewed_party_id))
            .collect();

        let mut missing = Vec::new();
        for (reviewer, _) in participations {
            for (reviewed, _) in participations {
                if reviewer != reviewed && !existing.contains(&(*reviewer, *reviewed)) {
                    missing.push((*reviewer, *reviewed));
                }
            }
        }

        Ok(missing)
    }

    async fn list(&self, criteria: &ReviewSearchCriteria) -> Result<ReviewListResult, DomainError> {
        let reviews = sqlx::query_as!(
            ReviewRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                reviewer_party_id as "reviewer_party_id!",
                reviewed_party_id as "reviewed_party_id!",
                reviewed_role as "reviewed_role!",
                overall_rating as "overall_rating!",
                communication_rating as "communication_rating: _",
                reliability_rating as "reliability_rating: _",
                quality_rating as "quality_rating: _",
                timeliness_rating as "timeliness_rating: _",
                review_text,
                is_verified as "is_verified!",
                is_public as "is_public!",
                platform_response,
                created_at as "created_at!"
            FROM reviews
            WHERE ($1::uuid IS NULL OR deal_id = $1)
              AND ($2::uuid IS NULL OR reviewer_party_id = $2)
              AND ($3::uuid IS NULL OR reviewed_party_id = $3)
              AND ($4::bool IS NULL OR is_public = $4)
            ORDER BY created_at DESC
            LIMIT $5
            OFFSET $6
            "#,
            criteria.deal_id,
            criteria.reviewer_party_id,
            criteria.reviewed_party_id,
            criteria.is_public,
            criteria.limit,
            criteria.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = self.count(criteria).await?;

        Ok(ReviewListResult {
            reviews: reviews.into_iter().map(build_review_from_row).collect(),
            total,
            limit: criteria.limit,
            offset: criteria.offset,
        })
    }

    async fn count(&self, criteria: &ReviewSearchCriteria) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM reviews
            WHERE ($1::uuid IS NULL OR deal_id = $1)
              AND ($2::uuid IS NULL OR reviewer_party_id = $2)
              AND ($3::uuid IS NULL OR reviewed_party_id = $3)
              AND ($4::bool IS NULL OR is_public = $4)
            "#,
            criteria.deal_id,
            criteria.reviewer_party_id,
            criteria.reviewed_party_id,
            criteria.is_public
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn update(&self, review: &Review) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE reviews
            SET overall_rating = $1,
                communication_rating = $2,
                reliability_rating = $3,
                quality_rating = $4,
                timeliness_rating = $5,
                review_text = $6,
                is_public = $7,
                platform_response = $8
            WHERE id = $9
            "#,
            review.overall_rating.value() as i32,
            review.communication_rating.map(|r| r.value() as i32),
            review.reliability_rating.map(|r| r.value() as i32),
            review.quality_rating.map(|r| r.value() as i32),
            review.timeliness_rating.map(|r| r.value() as i32),
            review.review_text,
            review.is_public,
            review.platform_response,
            review.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn hide(&self, id: Uuid, platform_response: Option<String>) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE reviews
            SET is_public = false,
                review_text = NULL,
                platform_response = $1
            WHERE id = $2
            "#,
            platform_response,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct ExistingReviewPair {
    reviewer_party_id: Uuid,
    reviewed_party_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct ReviewRow {
    id: Uuid,
    deal_id: Uuid,
    reviewer_party_id: Uuid,
    reviewed_party_id: Uuid,
    reviewed_role: String,
    overall_rating: i32,
    communication_rating: Option<i32>,
    reliability_rating: Option<i32>,
    quality_rating: Option<i32>,
    timeliness_rating: Option<i32>,
    review_text: Option<String>,
    is_verified: bool,
    is_public: bool,
    platform_response: Option<String>,
    created_at: OffsetDateTime,
}

fn build_review_from_row(row: ReviewRow) -> Review {
    Review {
        id: row.id,
        deal_id: row.deal_id,
        reviewer_party_id: row.reviewer_party_id,
        reviewed_party_id: row.reviewed_party_id,
        reviewed_role: DealRole::try_from(row.reviewed_role.as_str())
            .expect("database contains valid deal roles"),
        overall_rating: ReviewRating::new(row.overall_rating)
            .expect("database contains valid ratings"),
        communication_rating: row
            .communication_rating
            .and_then(|v| ReviewRating::new(v).ok()),
        reliability_rating: row
            .reliability_rating
            .and_then(|v| ReviewRating::new(v).ok()),
        quality_rating: row.quality_rating.and_then(|v| ReviewRating::new(v).ok()),
        timeliness_rating: row
            .timeliness_rating
            .and_then(|v| ReviewRating::new(v).ok()),
        review_text: row.review_text,
        is_verified: row.is_verified,
        is_public: row.is_public,
        platform_response: row.platform_response,
        created_at: row.created_at,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    if let SqlxError::Database(db_err) = &err {
        if db_err.constraint() == Some("idx_reviews_unique_pair") {
            return DomainError::DuplicateReview;
        }
    }
    DomainError::RepositoryError(err.to_string())
}
