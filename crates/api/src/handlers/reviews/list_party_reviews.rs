use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::reviews::dto::ListPartyReviewsQuery as AppQuery;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::reviews::dto::ReviewsResponse;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct Query {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_party_reviews(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<Query>,
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

    let app_query = AppQuery {
        actor_user_id: ctx.user_id,
        actor_party_id,
        is_admin,
        limit: query.limit.unwrap_or(20).clamp(1, 100),
        offset: query.offset.unwrap_or(0).max(0),
    };

    let result = state
        .list_party_reviews
        .execute(path.into_inner(), app_query)
        .await?;

    Ok(HttpResponse::Ok().json(ReviewsResponse::from(result)))
}
