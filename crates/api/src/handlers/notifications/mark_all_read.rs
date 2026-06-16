use crate::errors::ApiError;
use crate::handlers::notifications::{actor_party_id, extract_ctx};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::notifications::dto::MarkAllReadBody;

pub async fn mark_all_read(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<MarkAllReadBody>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    crate::middleware::auth::require_scope(&ctx, "notifications:write")?;

    let party_id = actor_party_id(&req);
    let count = state
        .mark_all_notifications_read
        .execute(ctx.user_id, party_id, body.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "marked_read": count })))
}
