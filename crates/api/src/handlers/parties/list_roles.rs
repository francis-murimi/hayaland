use crate::dto::PartyRolesResponse;
use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use uuid::Uuid;

pub async fn list_roles(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let roles = state.list_party_roles.execute(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(PartyRolesResponse { roles }))
}
