use async_trait::async_trait;
use domain::entities::{Enhancement, GeoPoint, Need, NeedPriority, Resource, ResourceCondition};
use domain::errors::DomainError;
use domain::repositories::{
    AdminFlags, CatalogItemStatus, CatalogItemType, CatalogListResult, CatalogRepository,
    CatalogSearchCriteria, CatalogSort, CategoryItemCounts,
};
use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, PgPool, Postgres, QueryBuilder};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresCatalogRepository {
    pool: PgPool,
}

impl PostgresCatalogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl CatalogRepository for PostgresCatalogRepository {
    async fn create_resource(&self, resource: &Resource) -> Result<(), DomainError> {
        let (lat, lng) = resource
            .location
            .map(|l| (Some(l.latitude), Some(l.longitude)))
            .unwrap_or((None, None));
        let condition = resource.condition.as_ref().map(|c| c.as_str());

        sqlx::query!(
            r#"
            INSERT INTO resources (
                id, deal_id, catalog_item_id, supplier_party_id, resource_type_id,
                resource_name, description, quantity, quantity_unit, condition,
                location_geo, availability_start, availability_end, document_urls,
                opportunity_cost, verified_by_platform, metadata,
                is_active, deal_count, platform_hidden, platform_featured,
                admin_notes, admin_reviewed_at, admin_reviewed_by,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                CASE
                    WHEN $11::float8 IS NOT NULL AND $12::float8 IS NOT NULL
                    THEN ST_SetSRID(ST_MakePoint($12, $11), 4326)::geography
                    ELSE NULL
                END,
                $13, $14, $15, $16, $17, $18,
                $19, $20, $21, $22, $23, $24, $25, $26, $27
            )
            "#,
            resource.id,
            resource.deal_id,
            resource.catalog_item_id,
            resource.supplier_party_id,
            resource.resource_type_id,
            resource.resource_name,
            resource.description,
            resource.quantity,
            resource.quantity_unit,
            condition,
            lat,
            lng,
            resource.availability_start,
            resource.availability_end,
            resource.document_urls.as_slice(),
            resource.opportunity_cost,
            resource.verified_by_platform,
            resource.metadata,
            resource.is_active,
            resource.deal_count,
            resource.platform_hidden,
            resource.platform_featured,
            resource.admin_notes,
            resource.admin_reviewed_at,
            resource.admin_reviewed_by,
            resource.created_at,
            resource.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_resource(&self, resource: &Resource) -> Result<(), DomainError> {
        let (lat, lng) = resource
            .location
            .map(|l| (Some(l.latitude), Some(l.longitude)))
            .unwrap_or((None, None));
        let condition = resource.condition.as_ref().map(|c| c.as_str());

        let result = sqlx::query!(
            r#"
            UPDATE resources
            SET resource_name = $1,
                description = $2,
                quantity = $3,
                quantity_unit = $4,
                condition = $5,
                location_geo = CASE
                    WHEN $6::float8 IS NOT NULL AND $7::float8 IS NOT NULL
                    THEN ST_SetSRID(ST_MakePoint($7, $6), 4326)::geography
                    ELSE NULL
                END,
                availability_start = $8,
                availability_end = $9,
                document_urls = $10,
                opportunity_cost = $11,
                verified_by_platform = $12,
                metadata = $13,
                is_active = $14,
                updated_at = $15
            WHERE id = $16
            "#,
            resource.resource_name,
            resource.description,
            resource.quantity,
            resource.quantity_unit,
            condition,
            lat,
            lng,
            resource.availability_start,
            resource.availability_end,
            resource.document_urls.as_slice(),
            resource.opportunity_cost,
            resource.verified_by_platform,
            resource.metadata,
            resource.is_active,
            resource.updated_at,
            resource.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::ResourceNotFound);
        }

        Ok(())
    }

    async fn delete_resource(&self, id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"DELETE FROM resources WHERE id = $1 AND deal_count = 0"#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if result.rows_affected() == 0 {
            let exists = sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM resources WHERE id = $1) as "exists!""#,
                id
            )
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

            if exists {
                return Err(DomainError::CatalogItemHasActiveDeals);
            }
            return Err(DomainError::ResourceNotFound);
        }

        Ok(())
    }

    async fn find_resource_by_id(&self, id: Uuid) -> Result<Option<Resource>, DomainError> {
        let row = sqlx::query_as!(
            ResourceRow,
            r#"
            SELECT
                id,
                deal_id,
                catalog_item_id,
                supplier_party_id,
                resource_type_id,
                resource_name,
                description,
                quantity,
                quantity_unit,
                condition,
                ST_Y(location_geo::geometry) as latitude,
                ST_X(location_geo::geometry) as longitude,
                availability_start,
                availability_end,
                document_urls,
                opportunity_cost,
                verified_by_platform,
                metadata,
                is_active,
                deal_count,
                platform_hidden,
                platform_featured,
                admin_notes,
                admin_reviewed_at,
                admin_reviewed_by,
                created_at,
                updated_at
            FROM resources
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_resource_from_row))
    }

    async fn list_resources(
        &self,
        criteria: &CatalogSearchCriteria,
    ) -> Result<CatalogListResult<Resource>, DomainError> {
        let items = self.list_resources_internal(criteria).await?;
        let total = self.count_resources_internal(criteria).await?;

        Ok(CatalogListResult {
            items,
            total,
            limit: criteria.limit,
            offset: criteria.offset,
        })
    }

    async fn count_resources_for_party(&self, party_id: Uuid) -> Result<i64, DomainError> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM resources
            WHERE supplier_party_id = $1 AND deal_id IS NULL
            "#,
            party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(count)
    }
    async fn create_need(&self, need: &Need) -> Result<(), DomainError> {
        let (lat, lng) = need
            .location
            .map(|l| (Some(l.latitude), Some(l.longitude)))
            .unwrap_or((None, None));
        let priority = need.priority.as_ref().map(|p| p.as_str());

        sqlx::query!(
            r#"
            INSERT INTO needs (
                id, deal_id, catalog_item_id, consumer_party_id, need_category_id,
                need_description, required_quantity, quantity_unit, quality_requirements,
                required_by_date, max_budget, budget_currency, estimated_fulfillment_value,
                acceptable_variants, priority, location_geo, metadata,
                is_active, deal_count, platform_hidden, platform_featured,
                admin_notes, admin_reviewed_at, admin_reviewed_by,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15,
                CASE
                    WHEN $16::float8 IS NOT NULL AND $17::float8 IS NOT NULL
                    THEN ST_SetSRID(ST_MakePoint($17, $16), 4326)::geography
                    ELSE NULL
                END,
                $18, $19, $20, $21, $22, $23, $24, $25, $26, $27
            )
            "#,
            need.id,
            need.deal_id,
            need.catalog_item_id,
            need.consumer_party_id,
            need.need_category_id,
            need.need_description,
            need.required_quantity,
            need.quantity_unit,
            need.quality_requirements,
            need.required_by_date,
            need.max_budget,
            need.budget_currency,
            need.estimated_fulfillment_value,
            need.acceptable_variants,
            priority,
            lat,
            lng,
            need.metadata,
            need.is_active,
            need.deal_count,
            need.platform_hidden,
            need.platform_featured,
            need.admin_notes,
            need.admin_reviewed_at,
            need.admin_reviewed_by,
            need.created_at,
            need.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_need(&self, need: &Need) -> Result<(), DomainError> {
        let (lat, lng) = need
            .location
            .map(|l| (Some(l.latitude), Some(l.longitude)))
            .unwrap_or((None, None));
        let priority = need.priority.as_ref().map(|p| p.as_str());

        let result = sqlx::query!(
            r#"
            UPDATE needs
            SET need_description = $1,
                required_quantity = $2,
                quantity_unit = $3,
                quality_requirements = $4,
                required_by_date = $5,
                max_budget = $6,
                budget_currency = $7,
                estimated_fulfillment_value = $8,
                acceptable_variants = $9,
                priority = $10,
                location_geo = CASE
                    WHEN $11::float8 IS NOT NULL AND $12::float8 IS NOT NULL
                    THEN ST_SetSRID(ST_MakePoint($12, $11), 4326)::geography
                    ELSE NULL
                END,
                metadata = $13,
                is_active = $14,
                updated_at = $15
            WHERE id = $16
            "#,
            need.need_description,
            need.required_quantity,
            need.quantity_unit,
            need.quality_requirements,
            need.required_by_date,
            need.max_budget,
            need.budget_currency,
            need.estimated_fulfillment_value,
            need.acceptable_variants,
            priority,
            lat,
            lng,
            need.metadata,
            need.is_active,
            need.updated_at,
            need.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NeedNotFound);
        }

        Ok(())
    }

    async fn delete_need(&self, id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query!(r#"DELETE FROM needs WHERE id = $1 AND deal_count = 0"#, id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;

        if result.rows_affected() == 0 {
            let exists = sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM needs WHERE id = $1) as "exists!""#,
                id
            )
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

            if exists {
                return Err(DomainError::CatalogItemHasActiveDeals);
            }
            return Err(DomainError::NeedNotFound);
        }

        Ok(())
    }

    async fn find_need_by_id(&self, id: Uuid) -> Result<Option<Need>, DomainError> {
        let row = sqlx::query_as!(
            NeedRow,
            r#"
            SELECT
                id,
                deal_id,
                catalog_item_id,
                consumer_party_id,
                need_category_id,
                need_description,
                required_quantity,
                quantity_unit,
                quality_requirements,
                required_by_date,
                max_budget,
                budget_currency,
                estimated_fulfillment_value,
                acceptable_variants,
                priority,
                ST_Y(location_geo::geometry) as latitude,
                ST_X(location_geo::geometry) as longitude,
                metadata,
                is_active,
                deal_count,
                platform_hidden,
                platform_featured,
                admin_notes,
                admin_reviewed_at,
                admin_reviewed_by,
                created_at,
                updated_at
            FROM needs
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_need_from_row))
    }

    async fn list_needs(
        &self,
        criteria: &CatalogSearchCriteria,
    ) -> Result<CatalogListResult<Need>, DomainError> {
        let items = self.list_needs_internal(criteria).await?;
        let total = self.count_needs_internal(criteria).await?;

        Ok(CatalogListResult {
            items,
            total,
            limit: criteria.limit,
            offset: criteria.offset,
        })
    }

    async fn count_needs_for_party(&self, party_id: Uuid) -> Result<i64, DomainError> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM needs
            WHERE consumer_party_id = $1 AND deal_id IS NULL
            "#,
            party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(count)
    }

    async fn create_enhancement(&self, enhancement: &Enhancement) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO enhancements (
                id, deal_id, catalog_item_id, enhancer_party_id, enhancement_type_id,
                enhancement_name, description, input_quantity, quantity_unit,
                estimated_input_cost, service_duration_hours, estimated_completion_days,
                deliverables, prerequisites, is_complete, completed_at, metadata,
                is_active, deal_count, platform_hidden, platform_featured,
                admin_notes, admin_reviewed_at, admin_reviewed_by,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18,
                $19, $20, $21, $22, $23, $24, $25, $26
            )
            "#,
            enhancement.id,
            enhancement.deal_id,
            enhancement.catalog_item_id,
            enhancement.enhancer_party_id,
            enhancement.enhancement_type_id,
            enhancement.enhancement_name,
            enhancement.description,
            enhancement.input_quantity,
            enhancement.quantity_unit,
            enhancement.estimated_input_cost,
            enhancement.service_duration_hours,
            enhancement.estimated_completion_days,
            enhancement.deliverables,
            enhancement.prerequisites,
            enhancement.is_complete,
            enhancement.completed_at,
            enhancement.metadata,
            enhancement.is_active,
            enhancement.deal_count,
            enhancement.platform_hidden,
            enhancement.platform_featured,
            enhancement.admin_notes,
            enhancement.admin_reviewed_at,
            enhancement.admin_reviewed_by,
            enhancement.created_at,
            enhancement.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_enhancement(&self, enhancement: &Enhancement) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"
            UPDATE enhancements
            SET enhancement_name = $1,
                description = $2,
                input_quantity = $3,
                quantity_unit = $4,
                estimated_input_cost = $5,
                service_duration_hours = $6,
                estimated_completion_days = $7,
                deliverables = $8,
                prerequisites = $9,
                is_complete = $10,
                completed_at = $11,
                metadata = $12,
                is_active = $13,
                updated_at = $14
            WHERE id = $15
            "#,
            enhancement.enhancement_name,
            enhancement.description,
            enhancement.input_quantity,
            enhancement.quantity_unit,
            enhancement.estimated_input_cost,
            enhancement.service_duration_hours,
            enhancement.estimated_completion_days,
            enhancement.deliverables,
            enhancement.prerequisites,
            enhancement.is_complete,
            enhancement.completed_at,
            enhancement.metadata,
            enhancement.is_active,
            enhancement.updated_at,
            enhancement.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::EnhancementNotFound);
        }

        Ok(())
    }

    async fn delete_enhancement(&self, id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"DELETE FROM enhancements WHERE id = $1 AND deal_count = 0"#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if result.rows_affected() == 0 {
            let exists = sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM enhancements WHERE id = $1) as "exists!""#,
                id
            )
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

            if exists {
                return Err(DomainError::CatalogItemHasActiveDeals);
            }
            return Err(DomainError::EnhancementNotFound);
        }

        Ok(())
    }

    async fn find_enhancement_by_id(&self, id: Uuid) -> Result<Option<Enhancement>, DomainError> {
        let row = sqlx::query_as!(
            EnhancementRow,
            r#"
            SELECT
                id,
                deal_id,
                catalog_item_id,
                enhancer_party_id,
                enhancement_type_id,
                enhancement_name,
                description,
                input_quantity,
                quantity_unit,
                estimated_input_cost,
                service_duration_hours,
                estimated_completion_days,
                deliverables,
                prerequisites,
                is_complete,
                completed_at,
                metadata,
                is_active,
                deal_count,
                platform_hidden,
                platform_featured,
                admin_notes,
                admin_reviewed_at,
                admin_reviewed_by,
                created_at,
                updated_at
            FROM enhancements
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_enhancement_from_row))
    }

    async fn list_enhancements(
        &self,
        criteria: &CatalogSearchCriteria,
    ) -> Result<CatalogListResult<Enhancement>, DomainError> {
        let items = self.list_enhancements_internal(criteria).await?;
        let total = self.count_enhancements_internal(criteria).await?;

        Ok(CatalogListResult {
            items,
            total,
            limit: criteria.limit,
            offset: criteria.offset,
        })
    }

    async fn count_enhancements_for_party(&self, party_id: Uuid) -> Result<i64, DomainError> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM enhancements
            WHERE enhancer_party_id = $1 AND deal_id IS NULL
            "#,
            party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(count)
    }
    async fn update_resource_admin_flags(
        &self,
        id: Uuid,
        flags: AdminFlags,
    ) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"
            UPDATE resources
            SET platform_hidden = COALESCE($1, platform_hidden),
                platform_featured = COALESCE($2, platform_featured),
                admin_notes = COALESCE($3, admin_notes),
                admin_reviewed_by = COALESCE($4, admin_reviewed_by),
                admin_reviewed_at = CASE WHEN $4 IS NOT NULL THEN now() ELSE admin_reviewed_at END,
                updated_at = now()
            WHERE id = $5
            "#,
            flags.platform_hidden,
            flags.platform_featured,
            flags.admin_notes,
            flags.admin_reviewed_by,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::ResourceNotFound);
        }

        Ok(())
    }

    async fn update_need_admin_flags(
        &self,
        id: Uuid,
        flags: AdminFlags,
    ) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"
            UPDATE needs
            SET platform_hidden = COALESCE($1, platform_hidden),
                platform_featured = COALESCE($2, platform_featured),
                admin_notes = COALESCE($3, admin_notes),
                admin_reviewed_by = COALESCE($4, admin_reviewed_by),
                admin_reviewed_at = CASE WHEN $4 IS NOT NULL THEN now() ELSE admin_reviewed_at END,
                updated_at = now()
            WHERE id = $5
            "#,
            flags.platform_hidden,
            flags.platform_featured,
            flags.admin_notes,
            flags.admin_reviewed_by,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NeedNotFound);
        }

        Ok(())
    }

    async fn update_enhancement_admin_flags(
        &self,
        id: Uuid,
        flags: AdminFlags,
    ) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"
            UPDATE enhancements
            SET platform_hidden = COALESCE($1, platform_hidden),
                platform_featured = COALESCE($2, platform_featured),
                admin_notes = COALESCE($3, admin_notes),
                admin_reviewed_by = COALESCE($4, admin_reviewed_by),
                admin_reviewed_at = CASE WHEN $4 IS NOT NULL THEN now() ELSE admin_reviewed_at END,
                updated_at = now()
            WHERE id = $5
            "#,
            flags.platform_hidden,
            flags.platform_featured,
            flags.admin_notes,
            flags.admin_reviewed_by,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::EnhancementNotFound);
        }

        Ok(())
    }

    async fn increment_deal_count(
        &self,
        item_type: CatalogItemType,
        id: Uuid,
    ) -> Result<(), DomainError> {
        let result = match item_type {
            CatalogItemType::Resource => sqlx::query!(
                "UPDATE resources SET deal_count = deal_count + 1 WHERE id = $1",
                id
            )
            .execute(&self.pool)
            .await
            .map_err(map_err)?,
            CatalogItemType::Need => sqlx::query!(
                "UPDATE needs SET deal_count = deal_count + 1 WHERE id = $1",
                id
            )
            .execute(&self.pool)
            .await
            .map_err(map_err)?,
            CatalogItemType::Enhancement => sqlx::query!(
                "UPDATE enhancements SET deal_count = deal_count + 1 WHERE id = $1",
                id
            )
            .execute(&self.pool)
            .await
            .map_err(map_err)?,
        };

        if result.rows_affected() == 0 {
            return Err(match item_type {
                CatalogItemType::Resource => DomainError::ResourceNotFound,
                CatalogItemType::Need => DomainError::NeedNotFound,
                CatalogItemType::Enhancement => DomainError::EnhancementNotFound,
            });
        }

        Ok(())
    }

    async fn find_resources_by_deal(&self, deal_id: Uuid) -> Result<Vec<Resource>, DomainError> {
        let rows = sqlx::query_as!(
            ResourceRow,
            r#"
            SELECT
                id,
                deal_id,
                catalog_item_id,
                supplier_party_id,
                resource_type_id,
                resource_name,
                description,
                quantity,
                quantity_unit,
                condition,
                ST_Y(location_geo::geometry) as latitude,
                ST_X(location_geo::geometry) as longitude,
                availability_start,
                availability_end,
                document_urls,
                opportunity_cost,
                verified_by_platform,
                metadata,
                is_active,
                deal_count,
                platform_hidden,
                platform_featured,
                admin_notes,
                admin_reviewed_at,
                admin_reviewed_by,
                created_at,
                updated_at
            FROM resources
            WHERE deal_id = $1
            ORDER BY created_at
            "#,
            deal_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_resource_from_row).collect())
    }

    async fn find_needs_by_deal(&self, deal_id: Uuid) -> Result<Vec<Need>, DomainError> {
        let rows = sqlx::query_as!(
            NeedRow,
            r#"
            SELECT
                id,
                deal_id,
                catalog_item_id,
                consumer_party_id,
                need_category_id,
                need_description,
                required_quantity,
                quantity_unit,
                quality_requirements,
                required_by_date,
                max_budget,
                budget_currency,
                estimated_fulfillment_value,
                acceptable_variants,
                priority,
                ST_Y(location_geo::geometry) as latitude,
                ST_X(location_geo::geometry) as longitude,
                metadata,
                is_active,
                deal_count,
                platform_hidden,
                platform_featured,
                admin_notes,
                admin_reviewed_at,
                admin_reviewed_by,
                created_at,
                updated_at
            FROM needs
            WHERE deal_id = $1
            ORDER BY created_at
            "#,
            deal_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_need_from_row).collect())
    }

    async fn find_enhancements_by_deal(
        &self,
        deal_id: Uuid,
    ) -> Result<Vec<Enhancement>, DomainError> {
        let rows = sqlx::query_as!(
            EnhancementRow,
            r#"
            SELECT
                id,
                deal_id,
                catalog_item_id,
                enhancer_party_id,
                enhancement_type_id,
                enhancement_name,
                description,
                input_quantity,
                quantity_unit,
                estimated_input_cost,
                service_duration_hours,
                estimated_completion_days,
                deliverables,
                prerequisites,
                is_complete,
                completed_at,
                metadata,
                is_active,
                deal_count,
                platform_hidden,
                platform_featured,
                admin_notes,
                admin_reviewed_at,
                admin_reviewed_by,
                created_at,
                updated_at
            FROM enhancements
            WHERE deal_id = $1
            ORDER BY created_at
            "#,
            deal_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_enhancement_from_row).collect())
    }

    async fn count_active_items_by_category(
        &self,
        category_id: Uuid,
    ) -> Result<CategoryItemCounts, DomainError> {
        let resource_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM resources
            WHERE resource_type_id = $1
              AND is_active = true
              AND platform_hidden = false
              AND deal_id IS NULL
            "#,
            category_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let need_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM needs
            WHERE need_category_id = $1
              AND is_active = true
              AND platform_hidden = false
              AND deal_id IS NULL
            "#,
            category_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let enhancement_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM enhancements
            WHERE enhancement_type_id = $1
              AND is_active = true
              AND platform_hidden = false
              AND deal_id IS NULL
            "#,
            category_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(CategoryItemCounts {
            resource_count,
            need_count,
            enhancement_count,
        })
    }
}
impl PostgresCatalogRepository {
    async fn list_resources_internal(
        &self,
        criteria: &CatalogSearchCriteria,
    ) -> Result<Vec<Resource>, DomainError> {
        let mut builder = QueryBuilder::<Postgres>::new(
            r#"
            SELECT
                r.id, r.deal_id, r.catalog_item_id, r.supplier_party_id, r.resource_type_id,
                r.resource_name, r.description, r.quantity, r.quantity_unit, r.condition,
                ST_Y(r.location_geo::geometry) as latitude,
                ST_X(r.location_geo::geometry) as longitude,
                r.availability_start, r.availability_end, r.document_urls,
                r.opportunity_cost, r.verified_by_platform, r.metadata,
                r.is_active, r.deal_count, r.platform_hidden, r.platform_featured,
                r.admin_notes, r.admin_reviewed_at, r.admin_reviewed_by,
                r.created_at, r.updated_at
            FROM resources r
            "#,
        );

        if criteria.sort == CatalogSort::TrustScore {
            builder.push(" JOIN parties p_trust ON p_trust.id = r.supplier_party_id");
        }

        builder.push(" WHERE 1=1");
        push_resource_filters(&mut builder, criteria);
        push_order_by(
            &mut builder,
            criteria,
            "r",
            "r.resource_name || ' ' || COALESCE(r.description, '')",
        );
        builder.push(" LIMIT ");
        builder.push_bind(criteria.limit);
        builder.push(" OFFSET ");
        builder.push_bind(criteria.offset);

        let rows = builder
            .build_query_as::<ResourceRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok(rows.into_iter().map(build_resource_from_row).collect())
    }

    async fn count_resources_internal(
        &self,
        criteria: &CatalogSearchCriteria,
    ) -> Result<i64, DomainError> {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM resources r");

        if criteria.sort == CatalogSort::TrustScore {
            builder.push(" JOIN parties p_trust ON p_trust.id = r.supplier_party_id");
        }

        builder.push(" WHERE 1=1");
        push_resource_filters(&mut builder, criteria);

        let count = builder
            .build_query_scalar::<i64>()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        Ok(count)
    }

    async fn list_needs_internal(
        &self,
        criteria: &CatalogSearchCriteria,
    ) -> Result<Vec<Need>, DomainError> {
        let mut builder = QueryBuilder::<Postgres>::new(
            r#"
            SELECT
                n.id, n.deal_id, n.catalog_item_id, n.consumer_party_id, n.need_category_id,
                n.need_description, n.required_quantity, n.quantity_unit, n.quality_requirements,
                n.required_by_date, n.max_budget, n.budget_currency, n.estimated_fulfillment_value,
                n.acceptable_variants, n.priority,
                ST_Y(n.location_geo::geometry) as latitude,
                ST_X(n.location_geo::geometry) as longitude,
                n.metadata,
                n.is_active, n.deal_count, n.platform_hidden, n.platform_featured,
                n.admin_notes, n.admin_reviewed_at, n.admin_reviewed_by,
                n.created_at, n.updated_at
            FROM needs n
            "#,
        );

        if criteria.sort == CatalogSort::TrustScore {
            builder.push(" JOIN parties p_trust ON p_trust.id = n.consumer_party_id");
        }

        builder.push(" WHERE 1=1");
        push_need_filters(&mut builder, criteria);
        push_order_by(
            &mut builder,
            criteria,
            "n",
            "n.need_description || ' ' || COALESCE(n.quality_requirements, '')",
        );
        builder.push(" LIMIT ");
        builder.push_bind(criteria.limit);
        builder.push(" OFFSET ");
        builder.push_bind(criteria.offset);

        let rows = builder
            .build_query_as::<NeedRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok(rows.into_iter().map(build_need_from_row).collect())
    }

    async fn count_needs_internal(
        &self,
        criteria: &CatalogSearchCriteria,
    ) -> Result<i64, DomainError> {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM needs n");

        if criteria.sort == CatalogSort::TrustScore {
            builder.push(" JOIN parties p_trust ON p_trust.id = n.consumer_party_id");
        }

        builder.push(" WHERE 1=1");
        push_need_filters(&mut builder, criteria);

        let count = builder
            .build_query_scalar::<i64>()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        Ok(count)
    }

    async fn list_enhancements_internal(
        &self,
        criteria: &CatalogSearchCriteria,
    ) -> Result<Vec<Enhancement>, DomainError> {
        let mut builder = QueryBuilder::<Postgres>::new(
            r#"
            SELECT
                e.id, e.deal_id, e.catalog_item_id, e.enhancer_party_id, e.enhancement_type_id,
                e.enhancement_name, e.description, e.input_quantity, e.quantity_unit,
                e.estimated_input_cost, e.service_duration_hours, e.estimated_completion_days,
                e.deliverables, e.prerequisites, e.is_complete, e.completed_at, e.metadata,
                e.is_active, e.deal_count, e.platform_hidden, e.platform_featured,
                e.admin_notes, e.admin_reviewed_at, e.admin_reviewed_by,
                e.created_at, e.updated_at
            FROM enhancements e
            "#,
        );

        if criteria.sort == CatalogSort::TrustScore {
            builder.push(" JOIN parties p_trust ON p_trust.id = e.enhancer_party_id");
        }

        builder.push(" WHERE 1=1");
        push_enhancement_filters(&mut builder, criteria);
        push_order_by(
            &mut builder,
            criteria,
            "e",
            "e.enhancement_name || ' ' || COALESCE(e.description, '')",
        );
        builder.push(" LIMIT ");
        builder.push_bind(criteria.limit);
        builder.push(" OFFSET ");
        builder.push_bind(criteria.offset);

        let rows = builder
            .build_query_as::<EnhancementRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok(rows.into_iter().map(build_enhancement_from_row).collect())
    }

    async fn count_enhancements_internal(
        &self,
        criteria: &CatalogSearchCriteria,
    ) -> Result<i64, DomainError> {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM enhancements e");

        if criteria.sort == CatalogSort::TrustScore {
            builder.push(" JOIN parties p_trust ON p_trust.id = e.enhancer_party_id");
        }

        builder.push(" WHERE 1=1");
        push_enhancement_filters(&mut builder, criteria);

        let count = builder
            .build_query_scalar::<i64>()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        Ok(count)
    }
}
fn push_resource_filters(builder: &mut QueryBuilder<Postgres>, criteria: &CatalogSearchCriteria) {
    if let Some(party_id) = criteria.party_id {
        builder.push(" AND r.supplier_party_id = ");
        builder.push_bind(party_id);
    }

    if let Some(category_id) = criteria.category_id {
        builder.push(" AND r.resource_type_id = ");
        builder.push_bind(category_id);
    }

    if let Some(domain_id) = criteria.domain_category_id {
        builder.push(" AND r.resource_type_id IN (");
        push_category_descendants(builder, domain_id);
        builder.push(")");
    }

    if let Some(query) = &criteria.query {
        builder.push(" AND (r.resource_name || ' ' || COALESCE(r.description, '')) % ");
        builder.push_bind(query);
    }

    push_status_and_visibility_filters(builder, criteria, "r");

    if criteria.verified_only {
        builder.push(" AND r.verified_by_platform = true");
    }

    if criteria.featured_only {
        builder.push(" AND r.platform_featured = true");
    }

    if let Some(geo) = &criteria.geo {
        builder.push(" AND ST_DWithin(r.location_geo, ST_SetSRID(ST_MakePoint(");
        builder.push_bind(geo.longitude);
        builder.push(", ");
        builder.push_bind(geo.latitude);
        builder.push("), 4326)::geography, ");
        builder.push_bind(geo.radius_km * 1000.0);
        builder.push(")");
    }
}

fn push_need_filters(builder: &mut QueryBuilder<Postgres>, criteria: &CatalogSearchCriteria) {
    if let Some(party_id) = criteria.party_id {
        builder.push(" AND n.consumer_party_id = ");
        builder.push_bind(party_id);
    }

    if let Some(category_id) = criteria.category_id {
        builder.push(" AND n.need_category_id = ");
        builder.push_bind(category_id);
    }

    if let Some(domain_id) = criteria.domain_category_id {
        builder.push(" AND n.need_category_id IN (");
        push_category_descendants(builder, domain_id);
        builder.push(")");
    }

    if let Some(query) = &criteria.query {
        builder.push(" AND (n.need_description || ' ' || COALESCE(n.quality_requirements, '')) % ");
        builder.push_bind(query);
    }

    push_status_and_visibility_filters(builder, criteria, "n");

    if criteria.featured_only {
        builder.push(" AND n.platform_featured = true");
    }

    if let Some(geo) = &criteria.geo {
        builder.push(" AND ST_DWithin(n.location_geo, ST_SetSRID(ST_MakePoint(");
        builder.push_bind(geo.longitude);
        builder.push(", ");
        builder.push_bind(geo.latitude);
        builder.push("), 4326)::geography, ");
        builder.push_bind(geo.radius_km * 1000.0);
        builder.push(")");
    }
}

fn push_enhancement_filters(
    builder: &mut QueryBuilder<Postgres>,
    criteria: &CatalogSearchCriteria,
) {
    if let Some(party_id) = criteria.party_id {
        builder.push(" AND e.enhancer_party_id = ");
        builder.push_bind(party_id);
    }

    if let Some(category_id) = criteria.category_id {
        builder.push(" AND e.enhancement_type_id = ");
        builder.push_bind(category_id);
    }

    if let Some(domain_id) = criteria.domain_category_id {
        builder.push(" AND e.enhancement_type_id IN (");
        push_category_descendants(builder, domain_id);
        builder.push(")");
    }

    if let Some(query) = &criteria.query {
        builder.push(" AND (e.enhancement_name || ' ' || COALESCE(e.description, '')) % ");
        builder.push_bind(query);
    }

    push_status_and_visibility_filters(builder, criteria, "e");

    if criteria.featured_only {
        builder.push(" AND e.platform_featured = true");
    }

    if let Some(geo) = &criteria.geo {
        builder.push(" AND ST_DWithin(e.location_geo, ST_SetSRID(ST_MakePoint(");
        builder.push_bind(geo.longitude);
        builder.push(", ");
        builder.push_bind(geo.latitude);
        builder.push("), 4326)::geography, ");
        builder.push_bind(geo.radius_km * 1000.0);
        builder.push(")");
    }
}

fn push_status_and_visibility_filters(
    builder: &mut QueryBuilder<Postgres>,
    criteria: &CatalogSearchCriteria,
    alias: &str,
) {
    match criteria.status {
        Some(CatalogItemStatus::Active) | None => {
            if !criteria.include_inactive {
                builder.push(format!(" AND {}.is_active = true", alias));
            }
        }
        Some(CatalogItemStatus::Inactive) => {
            builder.push(format!(" AND {}.is_active = false", alias));
        }
        Some(CatalogItemStatus::All) => {}
    }

    if !criteria.include_hidden {
        builder.push(format!(" AND {}.platform_hidden = false", alias));
    }
}

fn push_category_descendants(builder: &mut QueryBuilder<Postgres>, domain_id: Uuid) {
    builder.push(
        r#"
        WITH RECURSIVE descendants AS (
            SELECT id FROM categories WHERE id = "#,
    );
    builder.push_bind(domain_id);
    builder.push(
        r#"
            UNION ALL
            SELECT c.id FROM categories c
            JOIN descendants d ON c.parent_category_id = d.id
        )
        SELECT id FROM descendants
        "#,
    );
}

fn push_order_by(
    builder: &mut QueryBuilder<Postgres>,
    criteria: &CatalogSearchCriteria,
    alias: &str,
    search_expr: &str,
) {
    match criteria.sort {
        CatalogSort::Newest => {
            builder.push(format!(" ORDER BY {}.created_at DESC", alias));
        }
        CatalogSort::TrustScore => {
            builder.push(" ORDER BY p_trust.trust_score DESC, ");
            builder.push(format!("{}.created_at DESC", alias));
        }
        CatalogSort::Nearest => {
            if let Some(geo) = &criteria.geo {
                builder.push(format!(
                    " ORDER BY ST_Distance({}.location_geo, ST_SetSRID(ST_MakePoint(",
                    alias
                ));
                builder.push_bind(geo.longitude);
                builder.push(", ");
                builder.push_bind(geo.latitude);
                builder.push("), 4326)::geography) ASC NULLS LAST");
            } else {
                builder.push(format!(" ORDER BY {}.created_at DESC", alias));
            }
        }
        CatalogSort::Relevance => {
            if let Some(query) = &criteria.query {
                builder.push(format!(" ORDER BY similarity({}, ", search_expr));
                builder.push_bind(query);
                builder.push(") DESC");
            } else {
                builder.push(format!(" ORDER BY {}.created_at DESC", alias));
            }
        }
    }
}
#[derive(sqlx::FromRow)]
struct ResourceRow {
    id: Uuid,
    deal_id: Option<Uuid>,
    catalog_item_id: Option<Uuid>,
    supplier_party_id: Uuid,
    resource_type_id: Uuid,
    resource_name: String,
    description: Option<String>,
    quantity: Decimal,
    quantity_unit: String,
    condition: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    availability_start: Option<time::Date>,
    availability_end: Option<time::Date>,
    document_urls: Option<Vec<String>>,
    opportunity_cost: Option<Decimal>,
    verified_by_platform: bool,
    metadata: Option<serde_json::Value>,
    is_active: bool,
    deal_count: i32,
    platform_hidden: bool,
    platform_featured: bool,
    admin_notes: Option<String>,
    admin_reviewed_at: Option<OffsetDateTime>,
    admin_reviewed_by: Option<Uuid>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct NeedRow {
    id: Uuid,
    deal_id: Option<Uuid>,
    catalog_item_id: Option<Uuid>,
    consumer_party_id: Uuid,
    need_category_id: Uuid,
    need_description: String,
    required_quantity: Decimal,
    quantity_unit: String,
    quality_requirements: Option<String>,
    required_by_date: Option<time::Date>,
    max_budget: Option<Decimal>,
    budget_currency: Option<String>,
    estimated_fulfillment_value: Option<Decimal>,
    acceptable_variants: Option<String>,
    priority: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    metadata: Option<serde_json::Value>,
    is_active: bool,
    deal_count: i32,
    platform_hidden: bool,
    platform_featured: bool,
    admin_notes: Option<String>,
    admin_reviewed_at: Option<OffsetDateTime>,
    admin_reviewed_by: Option<Uuid>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct EnhancementRow {
    id: Uuid,
    deal_id: Option<Uuid>,
    catalog_item_id: Option<Uuid>,
    enhancer_party_id: Uuid,
    enhancement_type_id: Uuid,
    enhancement_name: String,
    description: Option<String>,
    input_quantity: Option<Decimal>,
    quantity_unit: Option<String>,
    estimated_input_cost: Option<Decimal>,
    service_duration_hours: Option<Decimal>,
    estimated_completion_days: Option<i32>,
    deliverables: Option<String>,
    prerequisites: Option<String>,
    is_complete: bool,
    completed_at: Option<OffsetDateTime>,
    metadata: Option<serde_json::Value>,
    is_active: bool,
    deal_count: i32,
    platform_hidden: bool,
    platform_featured: bool,
    admin_notes: Option<String>,
    admin_reviewed_at: Option<OffsetDateTime>,
    admin_reviewed_by: Option<Uuid>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn build_resource_from_row(row: ResourceRow) -> Resource {
    let mut resource = Resource::new(
        row.id,
        row.supplier_party_id,
        row.resource_type_id,
        row.resource_name,
        row.quantity,
        row.quantity_unit,
    )
    .expect("database contains a valid resource");

    resource.deal_id = row.deal_id;
    resource.catalog_item_id = row.catalog_item_id;
    resource.description = row.description;
    resource.condition = row
        .condition
        .and_then(|c| ResourceCondition::try_from(c.as_str()).ok());
    resource.location = match (row.latitude, row.longitude) {
        (Some(lat), Some(lng)) => GeoPoint::new(lat, lng).ok(),
        _ => None,
    };
    resource.availability_start = row.availability_start;
    resource.availability_end = row.availability_end;
    resource.document_urls = row.document_urls.unwrap_or_default();
    resource.opportunity_cost = row.opportunity_cost;
    resource.verified_by_platform = row.verified_by_platform;
    resource.metadata = row.metadata;
    resource.is_active = row.is_active;
    resource.deal_count = row.deal_count;
    resource.platform_hidden = row.platform_hidden;
    resource.platform_featured = row.platform_featured;
    resource.admin_notes = row.admin_notes;
    resource.admin_reviewed_at = row.admin_reviewed_at;
    resource.admin_reviewed_by = row.admin_reviewed_by;
    resource.created_at = row.created_at;
    resource.updated_at = row.updated_at;

    resource
}

fn build_need_from_row(row: NeedRow) -> Need {
    let mut need = Need::new(
        row.id,
        row.consumer_party_id,
        row.need_category_id,
        row.need_description,
        row.required_quantity,
        row.quantity_unit,
    )
    .expect("database contains a valid need");

    need.deal_id = row.deal_id;
    need.catalog_item_id = row.catalog_item_id;
    need.quality_requirements = row.quality_requirements;
    need.required_by_date = row.required_by_date;
    need.max_budget = row.max_budget;
    need.budget_currency = row.budget_currency.unwrap_or_else(|| "POINTS".to_string());
    need.estimated_fulfillment_value = row.estimated_fulfillment_value;
    need.acceptable_variants = row.acceptable_variants;
    need.priority = row
        .priority
        .and_then(|p| NeedPriority::try_from(p.as_str()).ok());
    need.location = match (row.latitude, row.longitude) {
        (Some(lat), Some(lng)) => GeoPoint::new(lat, lng).ok(),
        _ => None,
    };
    need.metadata = row.metadata;
    need.is_active = row.is_active;
    need.deal_count = row.deal_count;
    need.platform_hidden = row.platform_hidden;
    need.platform_featured = row.platform_featured;
    need.admin_notes = row.admin_notes;
    need.admin_reviewed_at = row.admin_reviewed_at;
    need.admin_reviewed_by = row.admin_reviewed_by;
    need.created_at = row.created_at;
    need.updated_at = row.updated_at;

    need
}

fn build_enhancement_from_row(row: EnhancementRow) -> Enhancement {
    let mut enhancement = Enhancement::new(
        row.id,
        row.enhancer_party_id,
        row.enhancement_type_id,
        row.enhancement_name,
    )
    .expect("database contains a valid enhancement");

    enhancement.deal_id = row.deal_id;
    enhancement.catalog_item_id = row.catalog_item_id;
    enhancement.description = row.description;
    enhancement.input_quantity = row.input_quantity;
    enhancement.quantity_unit = row.quantity_unit;
    enhancement.estimated_input_cost = row.estimated_input_cost;
    enhancement.service_duration_hours = row.service_duration_hours;
    enhancement.estimated_completion_days = row.estimated_completion_days;
    enhancement.deliverables = row.deliverables;
    enhancement.prerequisites = row.prerequisites;
    enhancement.is_complete = row.is_complete;
    enhancement.completed_at = row.completed_at;
    enhancement.metadata = row.metadata;
    enhancement.is_active = row.is_active;
    enhancement.deal_count = row.deal_count;
    enhancement.platform_hidden = row.platform_hidden;
    enhancement.platform_featured = row.platform_featured;
    enhancement.admin_notes = row.admin_notes;
    enhancement.admin_reviewed_at = row.admin_reviewed_at;
    enhancement.admin_reviewed_by = row.admin_reviewed_by;
    enhancement.created_at = row.created_at;
    enhancement.updated_at = row.updated_at;

    enhancement
}

fn map_err(err: SqlxError) -> DomainError {
    if let SqlxError::Database(db_err) = &err {
        match db_err.constraint() {
            Some("resources_supplier_party_id_fkey") => return DomainError::PartyNotFound,
            Some("needs_consumer_party_id_fkey") => return DomainError::PartyNotFound,
            Some("enhancements_enhancer_party_id_fkey") => return DomainError::PartyNotFound,
            _ => {}
        }
    }
    DomainError::RepositoryError(err.to_string())
}
