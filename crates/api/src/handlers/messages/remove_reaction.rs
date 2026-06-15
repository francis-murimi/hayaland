use crate::errors::ApiError;
use crate::handlers::messages::{actor_party_id, extract_ctx, is_message_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::messages::dto::ToggleReactionCommand;
use domain::entities::ReactionType;
use uuid::Uuid;

pub async fn remove_reaction(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, String)>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    let actor_party_id = actor_party_id(&req);
    let (message_id, reaction_type) = path.into_inner();

    let reaction_type = ReactionType::try_from(reaction_type.as_str())
        .map_err(|e| ApiError::Application(e.into()))?;

    let cmd = ToggleReactionCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        scopes: ctx.scopes.clone(),
        is_admin: is_message_admin(&ctx),
        message_id,
        reaction_type,
    };

    let result = state.react.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}
