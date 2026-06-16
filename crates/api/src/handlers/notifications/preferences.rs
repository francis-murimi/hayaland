use crate::errors::ApiError;
use crate::handlers::notifications::extract_ctx;
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::notifications::dto::NotificationPreferencesDto;

pub async fn get_preferences(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    crate::middleware::auth::require_scope(&ctx, "notifications:read")?;

    let result = state
        .get_notification_preferences
        .execute(ctx.user_id)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn update_preferences(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<NotificationPreferencesDto>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    crate::middleware::auth::require_scope(&ctx, "notifications:write")?;

    let result = state
        .update_notification_preferences
        .execute(ctx.user_id, body.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
