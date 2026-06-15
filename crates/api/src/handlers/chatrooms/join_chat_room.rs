use crate::errors::ApiError;
use crate::handlers::chatrooms::{actor_party_id, extract_ctx, is_chatroom_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::chatrooms::dto::JoinChatRoomCommand;
use uuid::Uuid;

pub async fn join_chat_room(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    let actor_party_id = actor_party_id(&req);

    let cmd = JoinChatRoomCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        scopes: ctx.scopes.clone(),
        is_admin: is_chatroom_admin(&ctx),
        room_id: path.into_inner(),
    };

    let result = state.join_chat_room.execute(cmd).await?;
    Ok(HttpResponse::Created().json(result))
}
