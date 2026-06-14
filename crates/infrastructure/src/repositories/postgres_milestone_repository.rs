use async_trait::async_trait;
use domain::entities::{Milestone, MilestoneStatus};
use domain::errors::DomainError;
use domain::repositories::MilestoneRepository;
use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, PgPool};
use uuid::Uuid;

pub struct PostgresMilestoneRepository {
    pool: PgPool,
}

impl PostgresMilestoneRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MilestoneRepository for PostgresMilestoneRepository {
    async fn create(&self, milestone: &Milestone) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO milestones (
                id, deal_id, milestone_name, description, assigned_to_party_id,
                due_date, completion_criteria, milestone_status, completion_percentage,
                payment_trigger_amount, completed_at, verified_by_party_id,
                display_order, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
            )
            "#,
            milestone.id,
            milestone.deal_id,
            milestone.milestone_name,
            milestone.description,
            milestone.assigned_to_party_id,
            milestone.due_date,
            milestone.completion_criteria,
            milestone.milestone_status.as_str(),
            milestone.completion_percentage,
            milestone.payment_trigger_amount,
            milestone.completed_at,
            milestone.verified_by_party_id,
            milestone.display_order,
            milestone.created_at,
            milestone.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update(&self, milestone: &Milestone) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE milestones
            SET milestone_name = $1,
                description = $2,
                assigned_to_party_id = $3,
                due_date = $4,
                completion_criteria = $5,
                milestone_status = $6,
                completion_percentage = $7,
                payment_trigger_amount = $8,
                completed_at = $9,
                verified_by_party_id = $10,
                display_order = $11,
                updated_at = $12
            WHERE id = $13
            "#,
            milestone.milestone_name,
            milestone.description,
            milestone.assigned_to_party_id,
            milestone.due_date,
            milestone.completion_criteria,
            milestone.milestone_status.as_str(),
            milestone.completion_percentage,
            milestone.payment_trigger_amount,
            milestone.completed_at,
            milestone.verified_by_party_id,
            milestone.display_order,
            milestone.updated_at,
            milestone.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query!("DELETE FROM milestones WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Milestone>, DomainError> {
        let row = sqlx::query_as!(
            MilestoneRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                milestone_name as "milestone_name!",
                description,
                assigned_to_party_id as "assigned_to_party_id!",
                verified_by_party_id as "verified_by_party_id!",
                due_date,
                completion_criteria as "completion_criteria!",
                milestone_status as "milestone_status!",
                completion_percentage as "completion_percentage!",
                payment_trigger_amount,
                completed_at,
                display_order as "display_order!",
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM milestones
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_milestone_from_row))
    }

    async fn find_by_deal(
        &self,
        deal_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Milestone>, DomainError> {
        let rows = sqlx::query_as!(
            MilestoneRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                milestone_name as "milestone_name!",
                description,
                assigned_to_party_id as "assigned_to_party_id!",
                verified_by_party_id as "verified_by_party_id!",
                due_date,
                completion_criteria as "completion_criteria!",
                milestone_status as "milestone_status!",
                completion_percentage as "completion_percentage!",
                payment_trigger_amount,
                completed_at,
                display_order as "display_order!",
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM milestones
            WHERE deal_id = $1
            ORDER BY display_order ASC, created_at ASC
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

        Ok(rows.into_iter().map(build_milestone_from_row).collect())
    }

    async fn count_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM milestones WHERE deal_id = $1"#,
            deal_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn count_verified_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM milestones
            WHERE deal_id = $1 AND milestone_status = 'VERIFIED'
            "#,
            deal_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn count_by_status(&self, deal_id: Uuid, status: &str) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM milestones
            WHERE deal_id = $1 AND milestone_status = $2
            "#,
            deal_id,
            status
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }
}

#[derive(sqlx::FromRow)]
struct MilestoneRow {
    id: Uuid,
    deal_id: Uuid,
    milestone_name: String,
    description: Option<String>,
    assigned_to_party_id: Uuid,
    verified_by_party_id: Uuid,
    due_date: Option<time::Date>,
    completion_criteria: String,
    milestone_status: String,
    completion_percentage: Decimal,
    payment_trigger_amount: Option<Decimal>,
    completed_at: Option<time::OffsetDateTime>,
    display_order: i32,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn build_milestone_from_row(row: MilestoneRow) -> Milestone {
    Milestone {
        id: row.id,
        deal_id: row.deal_id,
        milestone_name: row.milestone_name,
        description: row.description,
        assigned_to_party_id: row.assigned_to_party_id,
        verified_by_party_id: row.verified_by_party_id,
        due_date: row.due_date,
        completion_criteria: row.completion_criteria,
        milestone_status: MilestoneStatus::try_from(row.milestone_status.as_str())
            .unwrap_or(MilestoneStatus::Pending),
        completion_percentage: row.completion_percentage,
        payment_trigger_amount: row.payment_trigger_amount,
        completed_at: row.completed_at,
        display_order: row.display_order,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
