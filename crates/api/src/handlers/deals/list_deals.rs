use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::deals::dto::ListDealsQuery;
use application::users::token::AuthContext;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::AppState;

pub async fn list_deals(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ListDealsQuery>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx).ok();
    let is_admin = ctx.has_scope("admin:deals") || ctx.has_scope("admin:*");

    let result = state
        .list_deals
        .execute(ctx.user_id, actor_party_id, query.into_inner(), is_admin)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
