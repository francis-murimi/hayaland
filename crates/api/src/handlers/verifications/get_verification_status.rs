use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use application::verifications::dto::GetVerificationStatusQuery;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::verifications::dto::VerificationStatusResponse;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn get_verification_status(
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

    require_scope_or_admin(&ctx, "verifications:read", "admin:verifications")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let is_admin = ctx.has_scope("admin:verifications") || ctx.has_scope("admin:*");

    let query = GetVerificationStatusQuery {
        actor_user_id: ctx.user_id,
        actor_party_id,
        is_admin,
    };

    let result = state
        .get_verification_status
        .execute(path.into_inner(), query)
        .await?;

    Ok(HttpResponse::Ok().json(VerificationStatusResponse::from(result)))
}
