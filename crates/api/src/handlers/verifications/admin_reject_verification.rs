use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use application::verifications::dto::RejectVerificationCommand;
use uuid::Uuid;
use validator::Validate;

use crate::errors::ApiError;
use crate::handlers::verifications::dto::{RejectVerificationRequest, VerificationResponse};
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn admin_reject_verification(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<RejectVerificationRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "admin:verifications", "admin:verifications")?;

    let cmd = RejectVerificationCommand {
        actor_user_id: ctx.user_id,
        verification_id: path.into_inner(),
        reason: body.reason.clone(),
        review_notes: body.review_notes.clone(),
    };

    let result = state.reject_verification.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(VerificationResponse::from(result)))
}
