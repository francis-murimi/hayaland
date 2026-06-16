use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::disputes::dto::ListDealDisputesQuery;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::disputes::dto::DisputesResponse;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn list_deal_disputes(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<ListDealDisputesQuery>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "disputes:read", "admin:disputes")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let is_admin = ctx.has_scope("admin:disputes") || ctx.has_scope("admin:*");

    let mut q = query.into_inner();
    q.deal_id = path.into_inner();

    let result = state
        .list_deal_disputes
        .execute(q.deal_id, Some(actor_party_id), is_admin, q)
        .await?;
    Ok(HttpResponse::Ok().json(DisputesResponse::from(result)))
}
