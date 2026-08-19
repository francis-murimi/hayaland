use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::matching::dto::RespondToMatchCommand;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct RespondToMatchRequest {
    pub response: application::matching::dto::MatchResponseAction,
    pub notes: Option<String>,
    pub counter_value: Option<rust_decimal::Decimal>,
}

pub async fn respond_to_match(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<RespondToMatchRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "matches:write", "admin:matches")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;

    let cmd = RespondToMatchCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        match_suggestion_id: path.into_inner(),
        response: body.response,
        notes: body.notes.clone(),
        counter_value: body.counter_value,
    };

    state.respond_to_match.execute(cmd).await?;
    Ok(HttpResponse::NoContent().finish())
}
