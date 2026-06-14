use async_trait::async_trait;
use domain::entities::{Agreement, AgreementStatus, Signature, SignatureType};
use domain::errors::DomainError;
use domain::repositories::AgreementRepository;
use sqlx::{Error as SqlxError, PgPool};
use uuid::Uuid;

pub struct PostgresAgreementRepository {
    pool: PgPool,
}

impl PostgresAgreementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AgreementRepository for PostgresAgreementRepository {
    async fn create(&self, agreement: &Agreement) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO agreements (
                id, deal_id, agreement_status, agreement_text, governing_law,
                dispute_resolution, effective_date, termination_date, auto_renew,
                version, digital_signature_url, created_at, executed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
            agreement.id,
            agreement.deal_id,
            agreement.agreement_status.as_str(),
            agreement.agreement_text,
            agreement.governing_law,
            agreement.dispute_resolution,
            agreement.effective_date,
            agreement.termination_date,
            agreement.auto_renew,
            agreement.version,
            agreement.digital_signature_url,
            agreement.created_at,
            agreement.executed_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_deal_id(&self, deal_id: Uuid) -> Result<Option<Agreement>, DomainError> {
        let row = sqlx::query_as!(
            AgreementRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                agreement_status as "agreement_status!",
                agreement_text as "agreement_text!",
                governing_law,
                dispute_resolution,
                effective_date,
                termination_date,
                auto_renew as "auto_renew!",
                version as "version!",
                digital_signature_url,
                created_at as "created_at!",
                executed_at
            FROM agreements
            WHERE deal_id = $1
            "#,
            deal_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_agreement_from_row))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Agreement>, DomainError> {
        let row = sqlx::query_as!(
            AgreementRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                agreement_status as "agreement_status!",
                agreement_text as "agreement_text!",
                governing_law,
                dispute_resolution,
                effective_date,
                termination_date,
                auto_renew as "auto_renew!",
                version as "version!",
                digital_signature_url,
                created_at as "created_at!",
                executed_at
            FROM agreements
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_agreement_from_row))
    }

    async fn update(&self, agreement: &Agreement) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE agreements
            SET agreement_status = $1,
                agreement_text = $2,
                governing_law = $3,
                dispute_resolution = $4,
                effective_date = $5,
                termination_date = $6,
                auto_renew = $7,
                version = $8,
                digital_signature_url = $9,
                executed_at = $10
            WHERE id = $11
            "#,
            agreement.agreement_status.as_str(),
            agreement.agreement_text,
            agreement.governing_law,
            agreement.dispute_resolution,
            agreement.effective_date,
            agreement.termination_date,
            agreement.auto_renew,
            agreement.version,
            agreement.digital_signature_url,
            agreement.executed_at,
            agreement.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn create_signature(&self, signature: &Signature) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO signatures (
                id, agreement_id, party_id, signed_by_user_id, signature_type,
                signature_data, ip_address, signed_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            signature.id,
            signature.agreement_id,
            signature.party_id,
            signature.signed_by_user_id,
            signature.signature_type.as_str(),
            signature.signature_data,
            signature.ip_address,
            signature.signed_at,
            signature.version
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_signatures_by_agreement(
        &self,
        agreement_id: Uuid,
    ) -> Result<Vec<Signature>, DomainError> {
        let rows = sqlx::query_as!(
            SignatureRow,
            r#"
            SELECT
                id as "id!",
                agreement_id as "agreement_id!",
                party_id as "party_id!",
                signed_by_user_id as "signed_by_user_id!",
                signature_type as "signature_type!",
                signature_data as "signature_data!",
                ip_address,
                signed_at as "signed_at!",
                version as "version!"
            FROM signatures
            WHERE agreement_id = $1
            ORDER BY signed_at
            "#,
            agreement_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_signature_from_row).collect())
    }

    async fn has_party_signed(
        &self,
        agreement_id: Uuid,
        party_id: Uuid,
    ) -> Result<bool, DomainError> {
        let version = sqlx::query_scalar!(
            r#"
            SELECT version as "version!"
            FROM agreements
            WHERE id = $1
            "#,
            agreement_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        let version = match version {
            Some(v) => v,
            None => return Ok(false),
        };

        let exists = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM signatures
                WHERE agreement_id = $1 AND party_id = $2 AND version = $3
            ) as "exists!"
            "#,
            agreement_id,
            party_id,
            version
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(exists)
    }

    async fn count_signatures(&self, agreement_id: Uuid) -> Result<i64, DomainError> {
        let version = sqlx::query_scalar!(
            r#"
            SELECT version as "version!"
            FROM agreements
            WHERE id = $1
            "#,
            agreement_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        let version = match version {
            Some(v) => v,
            None => return Ok(0),
        };

        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM signatures
            WHERE agreement_id = $1 AND version = $2
            "#,
            agreement_id,
            version
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(count)
    }
}

#[derive(sqlx::FromRow)]
struct AgreementRow {
    id: Uuid,
    deal_id: Uuid,
    agreement_status: String,
    agreement_text: String,
    governing_law: Option<String>,
    dispute_resolution: Option<String>,
    effective_date: Option<time::Date>,
    termination_date: Option<time::Date>,
    auto_renew: bool,
    version: i32,
    digital_signature_url: Option<String>,
    created_at: time::OffsetDateTime,
    executed_at: Option<time::OffsetDateTime>,
}

#[derive(sqlx::FromRow)]
struct SignatureRow {
    id: Uuid,
    agreement_id: Uuid,
    party_id: Uuid,
    signed_by_user_id: Uuid,
    signature_type: String,
    signature_data: String,
    ip_address: Option<String>,
    signed_at: time::OffsetDateTime,
    version: i32,
}

fn build_agreement_from_row(row: AgreementRow) -> Agreement {
    Agreement {
        id: row.id,
        deal_id: row.deal_id,
        agreement_status: AgreementStatus::try_from(row.agreement_status.as_str())
            .expect("database should contain valid agreement statuses"),
        agreement_text: row.agreement_text,
        governing_law: row.governing_law,
        dispute_resolution: row.dispute_resolution,
        effective_date: row.effective_date,
        termination_date: row.termination_date,
        auto_renew: row.auto_renew,
        version: row.version,
        digital_signature_url: row.digital_signature_url,
        created_at: row.created_at,
        executed_at: row.executed_at,
    }
}

fn build_signature_from_row(row: SignatureRow) -> Signature {
    Signature {
        id: row.id,
        agreement_id: row.agreement_id,
        party_id: row.party_id,
        signed_by_user_id: row.signed_by_user_id,
        signature_type: SignatureType::try_from(row.signature_type.as_str())
            .expect("database should contain valid signature types"),
        signature_data: row.signature_data,
        ip_address: row.ip_address,
        signed_at: row.signed_at,
        version: row.version,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
