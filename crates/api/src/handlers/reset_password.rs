use crate::dto::ResetPasswordResponse;
use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::password_reset::dto::ResetPasswordCommand;
use validator::Validate;

pub async fn reset_password(
    state: web::Data<AppState>,
    body: web::Json<ResetPasswordRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let result = state
        .reset_password
        .execute(ResetPasswordCommand {
            token: body.token.clone(),
            password: body.password.clone(),
        })
        .await?;

    Ok(HttpResponse::Ok().json(ResetPasswordResponse {
        status: "password_reset".to_string(),
        user_id: result.user_id,
    }))
}

#[derive(Debug, serde::Deserialize, Validate)]
pub struct ResetPasswordRequest {
    pub token: String,
    #[validate(length(min = 8, message = "password must be at least 8 characters"))]
    pub password: String,
}
