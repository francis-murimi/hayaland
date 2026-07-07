use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::matching::dto::ListMatchesQuery;
use application::users::token::AuthContext;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::matches::services;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn list_matches(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ListMatchesQuery>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "matches:read", "admin:matches")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx).ok();
    let is_admin = ctx.has_scope("admin:matches") || ctx.has_scope("admin:*");

    let use_case = services::list_matches(state.db_pool.clone());
    let result = use_case
        .execute(ctx.user_id, actor_party_id, is_admin, query.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
