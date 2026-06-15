use crate::dto::ReactRequest;
use crate::errors::ApiError;
use crate::handlers::messages::{actor_party_id, extract_ctx, is_message_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::messages::dto::ToggleReactionCommand;
use uuid::Uuid;
use validator::Validate;

pub async fn react(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<ReactRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = extract_ctx(&req)?;
    let actor_party_id = actor_party_id(&req);

    let cmd = ToggleReactionCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        scopes: ctx.scopes.clone(),
        is_admin: is_message_admin(&ctx),
        message_id: path.into_inner(),
        reaction_type: body.reaction_type,
    };

    let result = state.react.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}
