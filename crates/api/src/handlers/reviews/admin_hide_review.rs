use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::middleware::auth::require_any_scope;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct HideRequest {
    #[serde(rename = "platformResponse")]
    pub platform_response: Option<String>,
}

pub async fn admin_hide_review(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<HideRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_any_scope(&ctx, &["admin:reviews", "admin:*"])?;

    state
        .hide_review
        .execute(path.into_inner(), body.platform_response.clone())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
