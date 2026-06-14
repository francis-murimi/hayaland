use crate::dto::{ListUsersQuery, UsersResponse};
use crate::errors::ApiError;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::users::dto::ListUsersQuery as AppListUsersQuery;
use application::users::token::AuthContext;
use validator::Validate;

pub async fn list_users(
    state: web::Data<AppState>,
    query: web::Query<ListUsersQuery>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    query.validate().map_err(ApiError::from)?;
    require_scope_or_admin(&ctx, "users:read", "admin:users")?;

    let app_query = AppListUsersQuery {
        page: query.page,
        per_page: query.per_page,
        active_only: query.active_only,
    };

    let result = state.list_users.execute(app_query).await?;
    Ok(HttpResponse::Ok().json(UsersResponse::from(result)))
}
