use crate::errors::ApiError;
use crate::handlers::notifications::{actor_party_id, extract_ctx};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::notifications::dto::UpdateNotificationBody;
use uuid::Uuid;

pub async fn mark_read(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<UpdateNotificationBody>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    crate::middleware::auth::require_scope(&ctx, "notifications:write")?;

    let party_id = actor_party_id(&req);
    state
        .mark_notification_read
        .execute(path.into_inner(), ctx.user_id, party_id, body.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
