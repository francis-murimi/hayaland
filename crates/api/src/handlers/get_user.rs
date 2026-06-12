use crate::dto::UserResponse;
use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use uuid::Uuid;

pub async fn get_user(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let user = state.get_user.execute(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(UserResponse::from(&user)))
}
