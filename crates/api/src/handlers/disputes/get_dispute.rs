use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::disputes::dto::GetDisputeQuery;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::disputes::dto::DisputeResponse;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn get_dispute(
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

    require_scope_or_admin(&ctx, "disputes:read", "admin:disputes")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let is_admin = ctx.has_scope("admin:disputes") || ctx.has_scope("admin:*");

    let query = GetDisputeQuery {
        dispute_id: path.into_inner(),
    };

    let result = state
        .get_dispute
        .execute(Some(actor_party_id), is_admin, query)
        .await?;
    Ok(HttpResponse::Ok().json(DisputeResponse::from(result)))
}
