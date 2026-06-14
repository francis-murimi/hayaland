use crate::dto::PartyRolesResponse;
use crate::errors::ApiError;
use crate::handlers::parties::is_admin;
use crate::AppState;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::parties::list_roles::ListPartyRolesQuery;
use application::users::token::AuthContext;
use uuid::Uuid;

pub async fn list_roles(
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

    let roles = state
        .list_party_roles
        .execute(
            path.into_inner(),
            ListPartyRolesQuery {
                actor_user_id: ctx.user_id,
                is_admin: is_admin(&ctx),
            },
        )
        .await?;
    Ok(HttpResponse::Ok().json(PartyRolesResponse { roles }))
}
