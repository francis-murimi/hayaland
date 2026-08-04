use crate::errors::ApiError;
use crate::middleware::auth::require_any_scope;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::users::token::AuthContext;

pub async fn get_dashboard_summary(
    state: web::Data<AppState>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    require_any_scope(&ctx, &["admin:analytics", "admin:*"])?;
    let summary = state.get_dashboard_summary.execute().await?;
    Ok(HttpResponse::Ok().json(summary))
}
