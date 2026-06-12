use crate::dto::UserResponse;
use crate::errors::ApiError;
use crate::middleware::auth::require_scope;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

pub async fn get_user(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    require_scope(&ctx, "users:read")?;
    let user = state.get_user.execute(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(UserResponse::from(&user)))
}
