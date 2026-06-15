use crate::dto::CreateChatRoomRequest;
use crate::errors::ApiError;
use crate::handlers::chatrooms::extract_ctx;
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::chatrooms::dto::CreateChatRoomCommand;
use validator::Validate;

pub async fn create_chat_room(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateChatRoomRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = extract_ctx(&req)?;

    let cmd = CreateChatRoomCommand {
        actor_user_id: ctx.user_id,
        scopes: ctx.scopes.clone(),
        name: body.name.clone(),
        description: body.description.clone(),
        room_type: body.room_type,
    };

    let result = state.create_chat_room.execute(cmd).await?;
    Ok(HttpResponse::Created().json(result))
}
