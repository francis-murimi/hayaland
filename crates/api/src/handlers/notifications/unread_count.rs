use crate::errors::ApiError;
use crate::handlers::notifications::{actor_party_id, extract_ctx};
use actix_web::{HttpRequest, HttpResponse};

pub async fn unread_count(
    state: actix_web::web::Data<crate::AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    crate::middleware::auth::require_scope(&ctx, "notifications:read")?;

    let party_id = actor_party_id(&req);
    let result = state
        .get_unread_notification_count
        .execute(ctx.user_id, party_id)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
