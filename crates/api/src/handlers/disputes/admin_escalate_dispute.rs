use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::disputes::dto::EscalateDisputeCommand;
use application::users::token::AuthContext;
use uuid::Uuid;
use validator::Validate;

use crate::errors::ApiError;
use crate::handlers::disputes::dto::{DisputeResponse, EscalateDisputeRequest};
use crate::middleware::auth::require_scope;
use crate::AppState;

pub async fn admin_escalate_dispute(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<EscalateDisputeRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope(&ctx, "admin:disputes")?;

    let cmd = EscalateDisputeCommand {
        actor_user_id: ctx.user_id,
        dispute_id: path.into_inner(),
        notes: body.notes.clone(),
    };

    let result = state.escalate_dispute.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(DisputeResponse::from(result)))
}
