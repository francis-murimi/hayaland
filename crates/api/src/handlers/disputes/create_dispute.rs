use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::disputes::dto::RaiseDisputeCommand;
use application::users::token::AuthContext;
use uuid::Uuid;
use validator::Validate;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::disputes::dto::{CreateDisputeRequest, DisputeResponse};
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn create_dispute(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<CreateDisputeRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "disputes:write", "admin:disputes")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let is_admin = ctx.has_scope("admin:disputes") || ctx.has_scope("admin:*");

    let cmd = RaiseDisputeCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        is_admin,
        deal_id: path.into_inner(),
        against_party_id: body.against_party_id,
        dispute_type: body.dispute_type.clone(),
        description: body.description.clone(),
        evidence_urls: body.evidence_urls.clone(),
    };

    let result = state.raise_dispute.execute(cmd).await?;
    Ok(HttpResponse::Created().json(DisputeResponse::from(result)))
}
