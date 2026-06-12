use crate::dto::{CreateUserRequest, CreateUserResponse};
use crate::errors::ApiError;
use crate::AppState;
use actix_web::{http::header::LOCATION, web, HttpResponse};
use application::users::dto::CreateUserCommand;
use validator::Validate;

pub async fn create_user(
    state: web::Data<AppState>,
    body: web::Json<CreateUserRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let cmd = CreateUserCommand {
        email: body.email.clone(),
        username: body.username.clone(),
        password: body.password.clone(),
    };

    let result = state.create_user.execute(cmd).await?;
    let location = format!("/api/v1/users/{}", result.id);

    Ok(HttpResponse::Created()
        .insert_header((LOCATION, location))
        .json(CreateUserResponse { id: result.id }))
}
