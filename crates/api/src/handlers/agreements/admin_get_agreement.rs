use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::AppState;

pub async fn admin_get_agreement(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    if !ctx.has_scope("admin:deals") && !ctx.has_scope("admin:*") {
        return Err(ApiError::Forbidden);
    }

    let result = state
        .get_agreement
        .execute(path.into_inner(), ctx.user_id, None, true)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
