use crate::errors::ApiError;
use crate::handlers::notifications::{extract_ctx, require_notification_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::notifications::dto::AdminSendNotificationRequest;

pub async fn admin_send(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<AdminSendNotificationRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    require_notification_admin(&ctx)?;

    let result = state
        .admin_send_notification
        .execute(ctx.user_id, body.into_inner())
        .await?;
    Ok(HttpResponse::Accepted().json(result))
}
