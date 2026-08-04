use crate::errors::ApiError;
use crate::middleware::auth::require_any_scope;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::audit_log::dto::RecordAdminActionCommand;
use application::users::token::AuthContext;

pub async fn record_admin_action(
    state: web::Data<AppState>,
    body: web::Json<RecordAdminActionCommand>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    require_any_scope(&ctx, &["admin:audit", "admin:*"])?;
    let mut cmd = body.into_inner();
    cmd.admin_user_id = ctx.user_id;
    let result = state.record_admin_action.execute(cmd).await?;
    Ok(HttpResponse::Created().json(result))
}
