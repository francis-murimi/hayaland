use crate::dto::UpdateChatRoomRequest;
use crate::errors::ApiError;
use crate::handlers::chatrooms::{extract_ctx, is_chatroom_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::chatrooms::dto::UpdateChatRoomCommand;
use uuid::Uuid;
use validator::Validate;

pub async fn update_chat_room(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<UpdateChatRoomRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = extract_ctx(&req)?;

    let cmd = UpdateChatRoomCommand {
        actor_user_id: ctx.user_id,
        scopes: ctx.scopes.clone(),
        is_admin: is_chatroom_admin(&ctx),
        room_id: path.into_inner(),
        name: body.name.clone(),
        description: body.description.clone(),
        room_type: body.room_type,
    };

    let result = state.update_chat_room.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}
