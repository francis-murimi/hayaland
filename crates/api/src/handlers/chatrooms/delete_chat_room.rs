use crate::errors::ApiError;
use crate::handlers::chatrooms::{extract_ctx, is_chatroom_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::chatrooms::dto::SoftDeleteChatRoomCommand;
use uuid::Uuid;

pub async fn delete_chat_room(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;

    let cmd = SoftDeleteChatRoomCommand {
        actor_user_id: ctx.user_id,
        scopes: ctx.scopes.clone(),
        is_admin: is_chatroom_admin(&ctx),
        room_id: path.into_inner(),
    };

    state.delete_chat_room.execute(cmd).await?;
    Ok(HttpResponse::NoContent().finish())
}
