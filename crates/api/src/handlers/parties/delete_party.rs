use crate::errors::ApiError;
use crate::handlers::parties::is_admin;
use crate::AppState;
use actix_web::HttpMessage;
use actix_web::{web, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

pub async fn delete_party(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    state
        .delete_party
        .execute(path.into_inner(), ctx.user_id, is_admin(&ctx))
        .await?;

    Ok(HttpResponse::NoContent().finish())
}
