use crate::dto::AdminBroadcastRequest;
use crate::errors::ApiError;
use crate::handlers::messages::{extract_ctx, require_message_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::messages::dto::AdminBroadcastCommand;
use validator::Validate;

pub async fn admin_broadcast(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<AdminBroadcastRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = extract_ctx(&req)?;
    require_message_admin(&ctx)?;

    let cmd = AdminBroadcastCommand {
        actor_user_id: ctx.user_id,
        scopes: ctx.scopes.clone(),
        target: body.target.clone(),
        subject: body.subject.clone(),
        content: body.content.clone(),
    };

    let results = state.admin_broadcast.execute(cmd).await?;
    Ok(HttpResponse::Created().json(results))
}
