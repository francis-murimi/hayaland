use async_trait::async_trait;
use domain::errors::DomainError;
use domain::repositories::{
    SearchRepository, SearchResult, SearchResultItem, SearchableCatalogItem, SearchableDeal,
    SearchableParty,
};
use sqlx::{Error as SqlxError, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresSearchRepository {
    pool: PgPool,
}

impl PostgresSearchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SearchRepository for PostgresSearchRepository {
    async fn search(
        &self,
        query: &str,
        target: domain::repositories::SearchTarget,
        limit: i64,
        offset: i64,
    ) -> Result<SearchResult, DomainError> {
        match target {
            domain::repositories::SearchTarget::Party => {
                self.search_parties(query, limit, offset).await
            }
            domain::repositories::SearchTarget::Resource => {
                self.search_resources(query, limit, offset).await
            }
            domain::repositories::SearchTarget::Need => {
                self.search_needs(query, limit, offset).await
            }
            domain::repositories::SearchTarget::Enhancement => {
                self.search_enhancements(query, limit, offset).await
            }
            domain::repositories::SearchTarget::Deal => {
                self.search_deals(query, limit, offset).await
            }
        }
    }
}

impl PostgresSearchRepository {
    async fn search_parties(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> Result<SearchResult, DomainError> {
        let rows = sqlx::query_as!(
            PartySearchRow,
            r#"
            SELECT
                id,
                display_name,
                party_type,
                verification_status,
                trust_score,
                primary_domain_id,
                is_active,
                created_at
            FROM parties
            WHERE (
                $1 = ''
                OR to_tsvector('english', COALESCE(display_name, '') || ' ' || COALESCE(email, ''))
                    @@ plainto_tsquery('english', $1)
            )
            AND is_active = true
            ORDER BY
                ts_rank(
                    to_tsvector('english', COALESCE(display_name, '') || ' ' || COALESCE(email, '')),
                    plainto_tsquery('english', $1)
                ) DESC,
                trust_score DESC,
                created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            query,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM parties
            WHERE (
                $1 = ''
                OR to_tsvector('english', COALESCE(display_name, '') || ' ' || COALESCE(email, ''))
                    @@ plainto_tsquery('english', $1)
            )
            AND is_active = true
            "#,
            query
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(SearchResult {
            items: rows.into_iter().map(Into::into).collect(),
            total,
            limit,
            offset,
        })
    }

    async fn search_resources(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> Result<SearchResult, DomainError> {
        let rows = sqlx::query_as!(
            CatalogSearchRow,
            r#"
            SELECT
                id,
                supplier_party_id AS owner_party_id,
                resource_type_id AS category_id,
                resource_name AS title,
                description,
                is_active,
                verified_by_platform,
                created_at
            FROM resources
            WHERE (
                $1 = ''
                OR to_tsvector('english', COALESCE(resource_name, '') || ' ' || COALESCE(description, ''))
                    @@ plainto_tsquery('english', $1)
            )
            AND is_active = true
            AND platform_hidden = false
            ORDER BY
                ts_rank(
                    to_tsvector('english', COALESCE(resource_name, '') || ' ' || COALESCE(description, '')),
                    plainto_tsquery('english', $1)
                ) DESC,
                created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            query,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM resources
            WHERE (
                $1 = ''
                OR to_tsvector('english', COALESCE(resource_name, '') || ' ' || COALESCE(description, ''))
                    @@ plainto_tsquery('english', $1)
            )
            AND is_active = true
            AND platform_hidden = false
            "#,
            query
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(SearchResult {
            items: rows
                .into_iter()
                .map(|r| SearchResultItem::Resource(map_catalog_row(r, "RESOURCE")))
                .collect(),
            total,
            limit,
            offset,
        })
    }

    async fn search_needs(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> Result<SearchResult, DomainError> {
        let rows = sqlx::query_as!(
            CatalogSearchRow,
            r#"
            SELECT
                id,
                consumer_party_id AS owner_party_id,
                need_category_id AS category_id,
                need_description AS title,
                quality_requirements AS description,
                is_active,
                false AS verified_by_platform,
                created_at
            FROM needs
            WHERE (
                $1 = ''
                OR to_tsvector('english', COALESCE(need_description, '') || ' ' || COALESCE(quality_requirements, ''))
                    @@ plainto_tsquery('english', $1)
            )
            AND is_active = true
            AND platform_hidden = false
            ORDER BY
                ts_rank(
                    to_tsvector('english', COALESCE(need_description, '') || ' ' || COALESCE(quality_requirements, '')),
                    plainto_tsquery('english', $1)
                ) DESC,
                created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            query,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM needs
            WHERE (
                $1 = ''
                OR to_tsvector('english', COALESCE(need_description, '') || ' ' || COALESCE(quality_requirements, ''))
                    @@ plainto_tsquery('english', $1)
            )
            AND is_active = true
            AND platform_hidden = false
            "#,
            query
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(SearchResult {
            items: rows
                .into_iter()
                .map(|r| SearchResultItem::Need(map_catalog_row(r, "NEED")))
                .collect(),
            total,
            limit,
            offset,
        })
    }

    async fn search_enhancements(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> Result<SearchResult, DomainError> {
        let rows = sqlx::query_as!(
            CatalogSearchRow,
            r#"
            SELECT
                id,
                enhancer_party_id AS owner_party_id,
                enhancement_type_id AS category_id,
                enhancement_name AS title,
                description,
                is_active,
                false AS verified_by_platform,
                created_at
            FROM enhancements
            WHERE (
                $1 = ''
                OR to_tsvector('english', COALESCE(enhancement_name, '') || ' ' || COALESCE(description, ''))
                    @@ plainto_tsquery('english', $1)
            )
            AND is_active = true
            AND platform_hidden = false
            ORDER BY
                ts_rank(
                    to_tsvector('english', COALESCE(enhancement_name, '') || ' ' || COALESCE(description, '')),
                    plainto_tsquery('english', $1)
                ) DESC,
                created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            query,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM enhancements
            WHERE (
                $1 = ''
                OR to_tsvector('english', COALESCE(enhancement_name, '') || ' ' || COALESCE(description, ''))
                    @@ plainto_tsquery('english', $1)
            )
            AND is_active = true
            AND platform_hidden = false
            "#,
            query
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(SearchResult {
            items: rows
                .into_iter()
                .map(|r| SearchResultItem::Enhancement(map_catalog_row(r, "ENHANCEMENT")))
                .collect(),
            total,
            limit,
            offset,
        })
    }

    async fn search_deals(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> Result<SearchResult, DomainError> {
        let rows = sqlx::query_as!(
            DealSearchRow,
            r#"
            SELECT
                id,
                deal_reference,
                deal_title,
                deal_description,
                deal_status,
                domain_category_id,
                initiator_party_id,
                is_public,
                created_at
            FROM deals
            WHERE (
                $1 = ''
                OR to_tsvector('english', COALESCE(deal_title, '') || ' ' || COALESCE(deal_description, ''))
                    @@ plainto_tsquery('english', $1)
            )
            AND is_public = true
            ORDER BY
                ts_rank(
                    to_tsvector('english', COALESCE(deal_title, '') || ' ' || COALESCE(deal_description, '')),
                    plainto_tsquery('english', $1)
                ) DESC,
                created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            query,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM deals
            WHERE (
                $1 = ''
                OR to_tsvector('english', COALESCE(deal_title, '') || ' ' || COALESCE(deal_description, ''))
                    @@ plainto_tsquery('english', $1)
            )
            AND is_public = true
            "#,
            query
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(SearchResult {
            items: rows.into_iter().map(Into::into).collect(),
            total,
            limit,
            offset,
        })
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}

fn map_catalog_row(row: CatalogSearchRow, item_type: &str) -> SearchableCatalogItem {
    SearchableCatalogItem {
        id: row.id,
        item_type: item_type.to_string(),
        owner_party_id: row.owner_party_id,
        category_id: row.category_id,
        title: row.title,
        description: row.description,
        is_active: row.is_active,
        verified_by_platform: row.verified_by_platform.unwrap_or(false),
        created_at: row.created_at,
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PartySearchRow {
    id: Uuid,
    display_name: String,
    party_type: String,
    verification_status: String,
    trust_score: f64,
    primary_domain_id: Option<Uuid>,
    is_active: bool,
    created_at: OffsetDateTime,
}

impl From<PartySearchRow> for SearchableParty {
    fn from(row: PartySearchRow) -> Self {
        Self {
            id: row.id,
            display_name: row.display_name,
            party_type: row.party_type,
            verification_status: row.verification_status,
            trust_score: row.trust_score,
            primary_domain_id: row.primary_domain_id,
            is_active: row.is_active,
            created_at: row.created_at,
        }
    }
}

impl From<PartySearchRow> for SearchResultItem {
    fn from(row: PartySearchRow) -> Self {
        SearchResultItem::Party(row.into())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct CatalogSearchRow {
    id: Uuid,
    owner_party_id: Uuid,
    category_id: Uuid,
    title: String,
    description: Option<String>,
    is_active: bool,
    verified_by_platform: Option<bool>,
    created_at: OffsetDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct DealSearchRow {
    id: Uuid,
    deal_reference: String,
    deal_title: String,
    deal_description: Option<String>,
    deal_status: String,
    domain_category_id: Uuid,
    initiator_party_id: Uuid,
    is_public: bool,
    created_at: OffsetDateTime,
}

impl From<DealSearchRow> for SearchableDeal {
    fn from(row: DealSearchRow) -> Self {
        Self {
            id: row.id,
            deal_reference: row.deal_reference,
            deal_title: row.deal_title,
            deal_description: row.deal_description,
            deal_status: row.deal_status,
            domain_category_id: row.domain_category_id,
            initiator_party_id: row.initiator_party_id,
            is_public: row.is_public,
            created_at: row.created_at,
        }
    }
}

impl From<DealSearchRow> for SearchResultItem {
    fn from(row: DealSearchRow) -> Self {
        SearchResultItem::Deal(row.into())
    }
}
