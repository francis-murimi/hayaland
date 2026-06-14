use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::deals::validate_deal::ValidateDealQuery;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn validate_deal(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "deals:read", "admin:deals")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx).ok();
    let deal_id = path.into_inner();

    let result = state
        .validate_deal
        .execute(
            deal_id,
            ValidateDealQuery {
                actor_user_id: ctx.user_id,
                actor_party_id,
                is_admin: ctx.has_scope("admin:deals") || ctx.has_scope("admin:*"),
            },
        )
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
