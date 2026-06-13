use crate::dto::PartyResponse;
use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use uuid::Uuid;

pub async fn get_party(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let result = state.get_party.execute(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(PartyResponse::from(result)))
}
