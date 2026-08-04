use crate::errors::ApiError;
use crate::middleware::auth::require_any_scope;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::audit_log::dto::AuditLogFiltersDto;
use application::users::token::AuthContext;

pub async fn list_audit_log(
    state: web::Data<AppState>,
    query: web::Query<AuditLogFiltersDto>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    require_any_scope(&ctx, &["admin:audit", "admin:*"])?;
    let result = state.list_audit_log.execute(query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
