use crate::dto::UserResponse;
use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::users::dto::DeactivateUserCommand;
use uuid::Uuid;

pub async fn deactivate_user(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let cmd = DeactivateUserCommand {
        id: path.into_inner(),
    };
    let user = state.deactivate_user.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(UserResponse::from(&user)))
}
