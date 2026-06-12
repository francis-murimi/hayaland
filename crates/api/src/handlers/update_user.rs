use crate::dto::{UpdateUserRequest, UserResponse};
use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::users::dto::UpdateUserCommand;
use uuid::Uuid;
use validator::Validate;

pub async fn update_user(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateUserRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let cmd = UpdateUserCommand {
        id: path.into_inner(),
        email: body.email.clone(),
        username: body.username.clone(),
    };

    let user = state.update_user.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(UserResponse::from(&user)))
}
