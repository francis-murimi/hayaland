use crate::dto::EditMessageRequest;
use crate::errors::ApiError;
use crate::handlers::messages::{actor_party_id, extract_ctx, is_message_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::messages::dto::EditMessageCommand;
use uuid::Uuid;
use validator::Validate;

pub async fn edit_message(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<EditMessageRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = extract_ctx(&req)?;
    let actor_party_id = actor_party_id(&req);

    let cmd = EditMessageCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        scopes: ctx.scopes.clone(),
        is_admin: is_message_admin(&ctx),
        message_id: path.into_inner(),
        content: body.content.clone(),
    };

    let result = state.edit_message.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}
