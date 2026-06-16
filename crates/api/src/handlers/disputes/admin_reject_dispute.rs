use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::disputes::dto::RejectDisputeCommand;
use application::users::token::AuthContext;
use uuid::Uuid;
use validator::Validate;

use crate::errors::ApiError;
use crate::handlers::disputes::dto::{DisputeResponse, RejectDisputeRequest};
use crate::middleware::auth::require_scope;
use crate::AppState;

pub async fn admin_reject_dispute(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<RejectDisputeRequest>,
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

    let cmd = RejectDisputeCommand {
        actor_user_id: ctx.user_id,
        dispute_id: path.into_inner(),
        reason: body.reason.clone(),
        next_deal_status: body.next_deal_status.clone(),
    };

    let result = state.reject_dispute.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(DisputeResponse::from(result)))
}
