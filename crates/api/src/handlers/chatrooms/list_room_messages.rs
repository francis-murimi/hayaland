use crate::dto::ListMessagesQueryParams;
use crate::errors::ApiError;
use crate::handlers::chatrooms::{actor_party_id, extract_ctx, is_chatroom_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::messages::dto::ListMessagesQuery;
use uuid::Uuid;

pub async fn list_room_messages(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<ListMessagesQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    let actor_party_id = actor_party_id(&req);
    let room_id = path.into_inner();

    let conversation = state
        .message_repository
        .find_room_conversation(room_id)
        .await
        .map_err(|e| ApiError::Application(e.into()))?
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::ConversationNotFound,
        ))?;

    let app_query = ListMessagesQuery {
        actor_user_id: ctx.user_id,
        actor_party_id,
        scopes: ctx.scopes.clone(),
        is_admin: is_chatroom_admin(&ctx),
        before_id: query.before_id,
        limit: query.limit.unwrap_or(50).clamp(1, 100),
    };

    let mut results = state
        .list_messages
        .execute(conversation.id, app_query)
        .await?;
    results.reverse();
    Ok(HttpResponse::Ok().json(results))
}
