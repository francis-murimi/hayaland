use crate::dto::{RegisterPushTokenRequest, RegisterPushTokenResponse};
use crate::errors::ApiError;
use crate::handlers::notifications::extract_ctx;
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::notifications::RegisterPushTokenCommand;
use validator::Validate;

pub async fn register_push_token(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<RegisterPushTokenRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    crate::middleware::auth::require_scope(&ctx, "notifications:write")?;
    body.validate().map_err(ApiError::from)?;

    let cmd = RegisterPushTokenCommand {
        device_token: body.device_token.clone(),
        provider: body.provider.clone(),
        device_type: body.device_type.clone(),
    };

    let token = state.register_push_token.execute(ctx.user_id, cmd).await?;

    Ok(HttpResponse::Created().json(RegisterPushTokenResponse { id: token.id }))
}
