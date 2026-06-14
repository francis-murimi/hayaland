use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::AppState;

pub async fn get_deal_wallet(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    let (party_id, deal_id) = path.into_inner();
    let result = state
        .get_deal_wallet
        .execute(ctx.user_id, party_id, deal_id)
        .await?;
    Ok(
        HttpResponse::Ok().json(crate::handlers::payments::dto::DealWalletResponse::from(
            result,
        )),
    )
}
