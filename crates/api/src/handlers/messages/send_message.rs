use crate::dto::SendMessageRequest;
use crate::errors::ApiError;
use crate::handlers::messages::{actor_party_id, extract_ctx, is_message_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::messages::dto::SendMessageCommand;
use validator::Validate;

pub async fn send_message(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<SendMessageRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = extract_ctx(&req)?;
    let actor_party_id = actor_party_id(&req);
    let is_admin = is_message_admin(&ctx);

    let cmd = SendMessageCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        scopes: ctx.scopes.clone(),
        is_admin,
        recipient_type: body.recipient_type,
        recipient_user_id: body.recipient_user_id,
        recipient_party_id: body.recipient_party_id,
        recipient_deal_id: body.recipient_deal_id,
        recipient_room_id: body.recipient_room_id,
        message_type: body.message_type,
        subject: body.subject.clone(),
        content: body.content.clone(),
        attachment_urls: body.attachment_urls.clone(),
        reply_to_message_id: body.reply_to_message_id,
    };

    let result = state.send_message.execute(cmd).await?;
    Ok(HttpResponse::Created().json(result))
}
