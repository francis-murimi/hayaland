use crate::dto::{UpdateUserRequest, UserResponse};
use crate::errors::ApiError;
use crate::middleware::auth::{require_owner_or_admin, require_scope_or_admin};
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::users::dto::UpdateUserCommand;
use application::users::token::AuthContext;
use uuid::Uuid;
use validator::Validate;

pub async fn update_user(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateUserRequest>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;
    require_scope_or_admin(&ctx, "users:write", "admin:users")?;

    let id = path.into_inner();
    require_owner_or_admin(&ctx, id)?;

    let cmd = UpdateUserCommand {
        id,
        email: body.email.clone(),
        username: body.username.clone(),
        roles: body.roles.clone(),
    };

    let user = state.update_user.execute(cmd, &ctx).await?;
    Ok(HttpResponse::Ok().json(UserResponse::from(&user)))
}
