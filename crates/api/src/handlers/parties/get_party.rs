use crate::dto::PartyResponse;
use crate::errors::ApiError;
use crate::handlers::parties::is_admin;
use crate::AppState;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::parties::get_party::GetPartyQuery;
use application::users::token::AuthContext;
use uuid::Uuid;

pub async fn get_party(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    let result = state
        .get_party
        .execute(
            path.into_inner(),
            GetPartyQuery {
                actor_user_id: ctx.user_id,
                is_admin: is_admin(&ctx),
            },
        )
        .await?;
    Ok(HttpResponse::Ok().json(PartyResponse::from(result)))
}
