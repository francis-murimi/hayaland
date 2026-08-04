use crate::errors::ApiError;
use crate::middleware::auth::require_any_scope;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::users::token::AuthContext;
use serde::Deserialize;
use time::Date;

#[derive(Debug, Deserialize)]
pub struct RefreshQuery {
    pub date: Option<Date>,
}

pub async fn refresh_daily_metrics(
    state: web::Data<AppState>,
    query: web::Query<RefreshQuery>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    require_any_scope(&ctx, &["admin:analytics", "admin:*"])?;
    let date = query
        .into_inner()
        .date
        .unwrap_or_else(|| time::OffsetDateTime::now_utc().date());
    state.refresh_daily_metrics.execute(date).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "refreshed": date })))
}
