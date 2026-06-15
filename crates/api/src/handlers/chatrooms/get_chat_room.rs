use crate::errors::ApiError;
use crate::handlers::chatrooms::{actor_party_ids, extract_ctx, is_chatroom_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use uuid::Uuid;

pub async fn get_chat_room(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    let party_ids = actor_party_ids(&req);

    let result = state
        .get_chat_room
        .execute(
            path.into_inner(),
            ctx.user_id,
            party_ids,
            is_chatroom_admin(&ctx),
        )
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
