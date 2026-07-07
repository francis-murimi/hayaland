use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::matches::services;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn get_status_counts(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "matches:read", "admin:matches")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;

    let use_case = services::admin_match_controls(state.db_pool.clone());
    let result = use_case.count_for_party(actor_party_id).await?;
    Ok(HttpResponse::Ok().json(result))
}
