use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use application::verifications::dto::ApproveVerificationCommand;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::verifications::dto::VerificationResponse;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn admin_approve_verification(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<serde_json::Value>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "admin:verifications", "admin:verifications")?;

    let review_notes = body
        .get("reviewNotes")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let cmd = ApproveVerificationCommand {
        actor_user_id: ctx.user_id,
        verification_id: path.into_inner(),
        review_notes,
    };

    let result = state.approve_verification.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(VerificationResponse::from(result)))
}
