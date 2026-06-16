use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::disputes::dto::ResolveDisputeCommand;
use application::users::token::AuthContext;
use uuid::Uuid;
use validator::Validate;

use crate::errors::ApiError;
use crate::handlers::disputes::dto::{DisputeResponse, ResolveDisputeRequest};
use crate::middleware::auth::require_scope;
use crate::AppState;

pub async fn admin_resolve_dispute(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<ResolveDisputeRequest>,
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

    let cmd = ResolveDisputeCommand {
        actor_user_id: ctx.user_id,
        dispute_id: path.into_inner(),
        resolution_type: body.resolution_type.clone(),
        resolution_outcome: body.resolution_outcome.clone(),
        severity: body.severity.clone(),
        resolution_notes: body.resolution_notes.clone(),
        next_deal_status: body.next_deal_status.clone(),
    };

    let result = state.resolve_dispute.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(DisputeResponse::from(result)))
}
