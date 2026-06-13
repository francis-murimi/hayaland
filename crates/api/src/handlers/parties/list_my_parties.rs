use crate::dto::MyPartiesResponse;
use crate::errors::ApiError;
use crate::AppState;
use actix_web::HttpMessage;
use actix_web::{web, HttpRequest, HttpResponse};
use application::users::token::AuthContext;

pub async fn list_my_parties(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    let parties = state.list_my_parties.execute(ctx.user_id).await?;
    Ok(HttpResponse::Ok().json(MyPartiesResponse { parties }))
}
