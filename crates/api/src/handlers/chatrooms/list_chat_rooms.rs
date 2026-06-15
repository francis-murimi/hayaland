use crate::dto::ChatRoomListQueryParams;
use crate::errors::ApiError;
use crate::handlers::chatrooms::extract_ctx;
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::chatrooms::dto::ChatRoomListQuery;
use domain::entities::ChatRoomType;
use validator::Validate;

pub async fn list_chat_rooms(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ChatRoomListQueryParams>,
) -> Result<HttpResponse, ApiError> {
    query.validate().map_err(ApiError::from)?;

    let ctx = extract_ctx(&req)?;

    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let page = query.page.unwrap_or(1).max(1);

    let room_type = match query.room_type.as_deref() {
        Some(s) => Some(ChatRoomType::try_from(s).map_err(|e| ApiError::Application(e.into()))?),
        None => None,
    };

    let app_query = ChatRoomListQuery {
        room_type,
        include_deleted: query.include_deleted,
        limit: per_page,
        offset: (page - 1) * per_page,
    };

    let results = state
        .list_chat_rooms
        .execute(ctx.user_id, app_query)
        .await?;
    Ok(HttpResponse::Ok().json(results))
}
