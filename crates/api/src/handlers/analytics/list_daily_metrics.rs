use crate::errors::ApiError;
use crate::middleware::auth::require_any_scope;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::analytics::dto::DateRangeQuery;
use application::users::token::AuthContext;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListMetricsQuery {
    #[serde(flatten)]
    pub date_range: DateRangeQuery,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_daily_metrics(
    state: web::Data<AppState>,
    query: web::Query<ListMetricsQuery>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    require_any_scope(&ctx, &["admin:analytics", "admin:*"])?;
    let q = query.into_inner();
    let result = state
        .list_daily_metrics
        .execute(q.date_range.from, q.date_range.to, q.limit, q.offset)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
