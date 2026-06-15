use crate::dto::UnreadCountResponse;
use crate::errors::ApiError;
use crate::handlers::messages::{actor_party_id, extract_ctx};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};

pub async fn unread_count(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    let actor_party_id = actor_party_id(&req);

    let count = state
        .get_unread_count
        .execute(ctx.user_id, actor_party_id)
        .await?;
    Ok(HttpResponse::Ok().json(UnreadCountResponse { count }))
}
