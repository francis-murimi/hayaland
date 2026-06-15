use crate::errors::ApiError;
use crate::handlers::messages::{actor_party_id, extract_ctx, is_message_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use uuid::Uuid;

pub async fn get_message(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    let actor_party_id = actor_party_id(&req);

    let result = state
        .get_message
        .execute(
            path.into_inner(),
            ctx.user_id,
            actor_party_id,
            is_message_admin(&ctx),
        )
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
