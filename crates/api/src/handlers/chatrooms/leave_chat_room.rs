use crate::errors::ApiError;
use crate::handlers::chatrooms::{actor_party_id, extract_ctx};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::chatrooms::dto::LeaveChatRoomCommand;
use uuid::Uuid;

pub async fn leave_chat_room(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    let actor_party_id = actor_party_id(&req);

    let cmd = LeaveChatRoomCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        room_id: path.into_inner(),
    };

    state.leave_chat_room.execute(cmd).await?;
    Ok(HttpResponse::NoContent().finish())
}
