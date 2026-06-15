use crate::errors::ApiError;
use crate::handlers::messages::{actor_party_id, extract_ctx, is_message_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::messages::dto::SoftDeleteMessageCommand;
use uuid::Uuid;

pub async fn delete_message(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    let actor_party_id = actor_party_id(&req);

    let cmd = SoftDeleteMessageCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        scopes: ctx.scopes.clone(),
        is_admin: is_message_admin(&ctx),
        message_id: path.into_inner(),
    };

    state.delete_message.execute(cmd).await?;
    Ok(HttpResponse::NoContent().finish())
}
