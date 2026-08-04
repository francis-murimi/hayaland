use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::media::dto::DeleteMediaCommand;
use application::users::token::AuthContext;
use uuid::Uuid;

pub async fn delete_media(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let is_admin = ctx.has_scope("admin:media") || ctx.has_scope("admin:*");
    let cmd = DeleteMediaCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: None,
        is_admin,
    };
    state.delete_media.execute(id, cmd).await?;
    Ok(HttpResponse::NoContent().finish())
}
