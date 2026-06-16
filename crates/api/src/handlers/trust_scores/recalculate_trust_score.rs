use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::middleware::auth::require_any_scope;
use crate::AppState;

pub async fn recalculate_trust_score(
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

    require_any_scope(&ctx, &["admin:trust", "admin:*"])?;

    let party_id = path.into_inner();
    let result = state
        .recalculate_trust_score
        .as_ref()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Infrastructure(
                "trust score service not configured".to_string(),
            ),
        ))?
        .execute(party_id)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
