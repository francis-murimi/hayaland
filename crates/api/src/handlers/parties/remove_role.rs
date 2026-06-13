use crate::errors::ApiError;
use crate::handlers::parties::{is_admin, parse_role};
use crate::AppState;
use actix_web::HttpMessage;
use actix_web::{web, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

pub async fn remove_role(
    state: web::Data<AppState>,
    path: web::Path<(Uuid, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    let (party_id, role_str) = path.into_inner();
    let role = parse_role(&role_str)?;

    state
        .remove_party_role
        .execute(party_id, role, ctx.user_id, is_admin(&ctx))
        .await?;

    Ok(HttpResponse::NoContent().finish())
}
