use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::AppState;

pub async fn get_trust_score(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let _ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    let party_id = path.into_inner();
    let result = state
        .get_trust_score
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
