use crate::dto::VerifyEmailResponse;
use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::email::dto::VerifyEmailCommand;

pub async fn verify_email(
    state: web::Data<AppState>,
    query: web::Query<VerifyEmailQuery>,
) -> Result<HttpResponse, ApiError> {
    let result = state
        .verify_email
        .execute(VerifyEmailCommand {
            token: query.token.clone(),
        })
        .await?;

    Ok(HttpResponse::Ok().json(VerifyEmailResponse {
        status: "verified".to_string(),
        user_id: result.user_id,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct VerifyEmailQuery {
    pub token: String,
}
