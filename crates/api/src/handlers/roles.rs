use crate::dto::{RolesResponse, UpdateRoleScopesRequest};
use crate::errors::ApiError;
use crate::middleware::auth::require_scope;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::roles::dto::UpdateRoleScopesCommand;
use application::users::token::AuthContext;
use validator::Validate;

pub async fn list_roles(
    state: web::Data<AppState>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    require_scope(&ctx, "users:admin")?;
    let roles = state.list_roles.execute().await?;
    Ok(HttpResponse::Ok().json(RolesResponse::from(roles)))
}

pub async fn update_role_scopes(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<UpdateRoleScopesRequest>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;
    require_scope(&ctx, "users:admin")?;

    let role = state
        .update_role_scopes
        .execute(UpdateRoleScopesCommand {
            name: path.into_inner(),
            scopes: body.scopes.clone(),
        })
        .await?;
    Ok(HttpResponse::Ok().json(role))
}
