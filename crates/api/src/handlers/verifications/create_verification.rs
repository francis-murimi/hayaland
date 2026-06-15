use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use application::verifications::dto::SubmitVerificationCommand;
use uuid::Uuid;
use validator::Validate;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::verifications::dto::{CreateVerificationRequest, VerificationResponse};
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn create_verification(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<CreateVerificationRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "verifications:write", "admin:verifications")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let is_admin = ctx.has_scope("admin:verifications") || ctx.has_scope("admin:*");

    let cmd = SubmitVerificationCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        target_party_id: path.into_inner(),
        is_admin,
        verification_type: body.verification_type.clone(),
        evidence_urls: body.evidence_urls.clone(),
        notes: body.notes.clone(),
    };

    let result = state.submit_verification.execute(cmd).await?;
    Ok(HttpResponse::Created().json(VerificationResponse::from(result)))
}
