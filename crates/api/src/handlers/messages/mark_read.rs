use crate::errors::ApiError;
use crate::handlers::messages::{actor_party_id, extract_ctx, is_message_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::messages::dto::MarkReadCommand;
use uuid::Uuid;

pub async fn mark_read(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    let actor_party_id = actor_party_id(&req);

    let cmd = MarkReadCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        scopes: ctx.scopes.clone(),
        is_admin: is_message_admin(&ctx),
        message_id: path.into_inner(),
    };

    let read = state.mark_read.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(read))
}
