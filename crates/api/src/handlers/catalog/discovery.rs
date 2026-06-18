use crate::dto::catalog::{
    DiscoveryDomainChild, DiscoveryDomainDetailResponse, DiscoveryDomainResponse,
    DiscoveryDomainsResponse,
};
use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::errors::ApplicationError;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow)]
struct DomainRow {
    id: Uuid,
    category_code: String,
    category_name: String,
    description: Option<String>,
}

#[derive(Debug, FromRow)]
struct ChildRow {
    id: Uuid,
    parent_category_id: Option<Uuid>,
    category_code: String,
    category_name: String,
    category_type: String,
}

fn map_sqlx(err: sqlx::Error) -> ApiError {
    ApiError::Application(ApplicationError::Infrastructure(err.to_string()))
}

pub async fn list_domains(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let domains = sqlx::query_as::<_, DomainRow>(
        r#"
        SELECT id, category_code, category_name, description
        FROM categories
        WHERE category_type = 'DOMAIN' AND is_active = true
        ORDER BY display_order, category_name
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(map_sqlx)?;

    let mut items = Vec::with_capacity(domains.len());
    for d in domains {
        let counts = domain_counts(&state.db_pool, d.id).await?;
        items.push(DiscoveryDomainResponse {
            id: d.id,
            category_code: d.category_code,
            category_name: d.category_name,
            description: d.description,
            resource_count: counts.resource_count,
            need_count: counts.need_count,
            enhancement_count: counts.enhancement_count,
        });
    }

    Ok(HttpResponse::Ok().json(DiscoveryDomainsResponse { domains: items }))
}

pub async fn get_domain(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let domain = sqlx::query_as::<_, DomainRow>(
        r#"
        SELECT id, category_code, category_name, description
        FROM categories
        WHERE id = $1 AND category_type = 'DOMAIN' AND is_active = true
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(map_sqlx)?
    .ok_or(ApiError::Application(ApplicationError::NotFound))?;

    let counts = domain_counts(&state.db_pool, id).await?;
    let children = sqlx::query_as::<_, ChildRow>(
        r#"
        WITH RECURSIVE descendants AS (
            SELECT id, parent_category_id, category_code, category_name, category_type
            FROM categories
            WHERE id = $1
            UNION ALL
            SELECT c.id, c.parent_category_id, c.category_code, c.category_name, c.category_type
            FROM categories c
            JOIN descendants d ON c.parent_category_id = d.id
        )
        SELECT id, parent_category_id, category_code, category_name, category_type
        FROM descendants
        WHERE id != $1
        ORDER BY category_type, category_name
        "#,
    )
    .bind(id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(map_sqlx)?;

    Ok(HttpResponse::Ok().json(DiscoveryDomainDetailResponse {
        id: domain.id,
        category_code: domain.category_code,
        category_name: domain.category_name,
        description: domain.description,
        resource_count: counts.resource_count,
        need_count: counts.need_count,
        enhancement_count: counts.enhancement_count,
        child_categories: children
            .into_iter()
            .map(|c| DiscoveryDomainChild {
                id: c.id,
                parent_category_id: c.parent_category_id,
                category_code: c.category_code,
                category_name: c.category_name,
                category_type: c.category_type,
            })
            .collect(),
    }))
}

#[derive(Debug, sqlx::FromRow)]
struct DomainCounts {
    resource_count: i64,
    need_count: i64,
    enhancement_count: i64,
}

async fn domain_counts(pool: &sqlx::PgPool, domain_id: Uuid) -> Result<DomainCounts, ApiError> {
    let row = sqlx::query_as::<_, DomainCounts>(
        r#"
        WITH RECURSIVE descendants AS (
            SELECT id FROM categories WHERE id = $1
            UNION ALL
            SELECT c.id FROM categories c JOIN descendants d ON c.parent_category_id = d.id
        )
        SELECT
            (SELECT COUNT(*) FROM resources r
             WHERE r.resource_type_id IN (SELECT id FROM descendants)
               AND r.is_active = true
               AND r.platform_hidden = false
               AND r.deal_id IS NULL) AS resource_count,
            (SELECT COUNT(*) FROM needs n
             WHERE n.need_category_id IN (SELECT id FROM descendants)
               AND n.is_active = true
               AND n.platform_hidden = false
               AND n.deal_id IS NULL) AS need_count,
            (SELECT COUNT(*) FROM enhancements e
             WHERE e.enhancement_type_id IN (SELECT id FROM descendants)
               AND e.is_active = true
               AND e.platform_hidden = false
               AND e.deal_id IS NULL) AS enhancement_count
        "#,
    )
    .bind(domain_id)
    .fetch_one(pool)
    .await
    .map_err(map_sqlx)?;

    Ok(row)
}
