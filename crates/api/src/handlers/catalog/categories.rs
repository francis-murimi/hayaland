use crate::dto::catalog::{CategoryNode, CategoryTreeResponse};
use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::errors::ApplicationError;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow)]
struct CategoryRow {
    id: Uuid,
    parent_category_id: Option<Uuid>,
    category_name: String,
    category_code: String,
    description: Option<String>,
    category_type: String,
    icon_url: Option<String>,
    #[allow(dead_code)]
    display_order: i32,
}

fn map_sqlx(err: sqlx::Error) -> ApiError {
    ApiError::Application(ApplicationError::Infrastructure(err.to_string()))
}

pub async fn list_resource_categories(
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    list_categories_by_type(state, Some("RESOURCE_TYPE")).await
}

pub async fn list_need_categories(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    list_categories_by_type(state, Some("NEED_TYPE")).await
}

pub async fn list_enhancement_categories(
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    list_categories_by_type(state, Some("ENHANCEMENT_TYPE")).await
}

pub async fn list_all_categories(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    list_categories_by_type(state, None).await
}

async fn list_categories_by_type(
    state: web::Data<AppState>,
    category_type: Option<&str>,
) -> Result<HttpResponse, ApiError> {
    let rows = if let Some(ct) = category_type {
        sqlx::query_as::<_, CategoryRow>(
            r#"
            SELECT id, parent_category_id, category_name, category_code, description,
                   category_type, icon_url, display_order
            FROM categories
            WHERE is_active = true AND category_type = $1
            ORDER BY display_order, category_name
            "#,
        )
        .bind(ct)
        .fetch_all(&state.db_pool)
        .await
        .map_err(map_sqlx)?
    } else {
        sqlx::query_as::<_, CategoryRow>(
            r#"
            SELECT id, parent_category_id, category_name, category_code, description,
                   category_type, icon_url, display_order
            FROM categories
            WHERE is_active = true
            ORDER BY display_order, category_name
            "#,
        )
        .fetch_all(&state.db_pool)
        .await
        .map_err(map_sqlx)?
    };

    let tree = build_tree(rows);
    Ok(HttpResponse::Ok().json(CategoryTreeResponse { categories: tree }))
}

fn build_tree(rows: Vec<CategoryRow>) -> Vec<CategoryNode> {
    let mut by_parent: std::collections::HashMap<Option<Uuid>, Vec<&CategoryRow>> =
        std::collections::HashMap::new();
    for row in &rows {
        by_parent
            .entry(row.parent_category_id)
            .or_default()
            .push(row);
    }

    fn build(
        parent: Option<Uuid>,
        by_parent: &std::collections::HashMap<Option<Uuid>, Vec<&CategoryRow>>,
    ) -> Vec<CategoryNode> {
        by_parent
            .get(&parent)
            .map(|children| {
                children
                    .iter()
                    .map(|row| CategoryNode {
                        id: row.id,
                        parent_category_id: row.parent_category_id,
                        category_name: row.category_name.clone(),
                        category_code: row.category_code.clone(),
                        description: row.description.clone(),
                        category_type: row.category_type.clone(),
                        icon_url: row.icon_url.clone(),
                        children: build(Some(row.id), by_parent),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    build(None, &by_parent)
}
