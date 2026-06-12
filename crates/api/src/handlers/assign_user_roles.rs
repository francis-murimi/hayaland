use crate::dto::AssignUserRolesRequest;
use crate::errors::ApiError;
use crate::middleware::auth::require_scope;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::roles::dto::AssignUserRolesCommand;
use application::users::token::AuthContext;
use uuid::Uuid;
use validator::Validate;

pub async fn assign_user_roles(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<AssignUserRolesRequest>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;
    require_scope(&ctx, "users:admin")?;

    let user = state
        .assign_user_roles
        .execute(AssignUserRolesCommand {
            user_id: path.into_inner(),
            roles: body.roles.clone(),
        })
        .await?;
    Ok(HttpResponse::Ok().json(user))
}
