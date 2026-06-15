use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::reviews::dto::DealReviewStatusResponse;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn get_deal_review_status(
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

    require_scope_or_admin(&ctx, "reviews:read", "admin:reviews")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx).ok();
    let is_admin = ctx.has_scope("admin:reviews") || ctx.has_scope("admin:*");

    let result = state
        .get_deal_review_status
        .execute(path.into_inner(), ctx.user_id, actor_party_id, is_admin)
        .await?;
    Ok(HttpResponse::Ok().json(DealReviewStatusResponse::from(result)))
}
