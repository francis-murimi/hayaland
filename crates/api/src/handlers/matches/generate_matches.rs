use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::matching::dto::GenerateMatchesCommand;
use application::users::token::AuthContext;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::matches::services;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn generate_matches(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<GenerateMatchesCommand>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "matches:write", "admin:matches")?;

    let is_admin = ctx.has_scope("admin:matches") || ctx.has_scope("admin:*");
    let actor_party_id = if is_admin {
        resolve_actor_party_id(&req, &ctx).ok()
    } else {
        Some(resolve_actor_party_id(&req, &ctx)?)
    };

    let mut cmd = body.into_inner();
    cmd.actor_user_id = ctx.user_id;
    cmd.actor_party_id = actor_party_id;
    cmd.is_admin = is_admin;

    let use_case = services::generate_matches(state.db_pool.clone());
    let result = use_case.execute(cmd).await?;
    Ok(HttpResponse::Created().json(result))
}
