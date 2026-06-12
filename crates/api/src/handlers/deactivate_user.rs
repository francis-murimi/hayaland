use crate::dto::UserResponse;
use crate::errors::ApiError;
use crate::middleware::auth::{require_owner_or_admin, require_scope};
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::users::dto::DeactivateUserCommand;
use application::users::token::AuthContext;
use uuid::Uuid;

pub async fn deactivate_user(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    require_scope(&ctx, "users:write")?;

    let id = path.into_inner();
    require_owner_or_admin(&ctx, id)?;

    let cmd = DeactivateUserCommand { id };
    let user = state.deactivate_user.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(UserResponse::from(&user)))
}
