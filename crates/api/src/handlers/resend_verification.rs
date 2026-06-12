use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::email::dto::ResendVerificationCommand;
use validator::Validate;

pub async fn resend_verification(
    state: web::Data<AppState>,
    body: web::Json<ResendVerificationRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    state
        .resend_verification_email
        .execute(ResendVerificationCommand {
            email: body.email.clone(),
        })
        .await?;

    Ok(HttpResponse::Accepted().finish())
}

#[derive(Debug, serde::Deserialize, Validate)]
pub struct ResendVerificationRequest {
    #[validate(email(message = "invalid email"))]
    pub email: String,
}
