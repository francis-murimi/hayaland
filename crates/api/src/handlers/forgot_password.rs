use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::password_reset::dto::RequestPasswordResetCommand;
use validator::Validate;

pub async fn forgot_password(
    state: web::Data<AppState>,
    body: web::Json<ForgotPasswordRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    state
        .request_password_reset
        .execute(RequestPasswordResetCommand {
            email: body.email.clone(),
        })
        .await?;

    Ok(HttpResponse::Accepted().finish())
}

#[derive(Debug, serde::Deserialize, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(email(message = "invalid email"))]
    pub email: String,
}
