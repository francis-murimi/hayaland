use crate::errors::ApiError;
use crate::handlers::notifications::{actor_party_id, extract_ctx};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use uuid::Uuid;

pub async fn get_notification(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    crate::middleware::auth::require_scope(&ctx, "notifications:read")?;

    let party_id = actor_party_id(&req);
    let result = state
        .get_notification
        .execute(path.into_inner(), ctx.user_id, party_id)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
