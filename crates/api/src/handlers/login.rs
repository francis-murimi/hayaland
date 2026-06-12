use crate::dto::{LoginRequest, LoginResponse};
use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::users::dto::AuthenticateUserCommand;
use validator::Validate;

pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let cmd = AuthenticateUserCommand {
        email: body.email.clone(),
        password: body.password.clone(),
    };

    let result = state.authenticate_user.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(LoginResponse::from(result)))
}
