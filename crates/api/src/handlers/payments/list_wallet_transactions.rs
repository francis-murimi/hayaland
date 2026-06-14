use crate::handlers::payments::dto::ListTransactionsQuery;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::AppState;

pub async fn list_wallet_transactions(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<ListTransactionsQuery>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    let result = state
        .list_wallet_transactions
        .execute(ctx.user_id, path.into_inner(), query.into_inner().into())
        .await?;
    Ok(
        HttpResponse::Ok().json(crate::handlers::payments::dto::TransactionsResponse::from(
            result,
        )),
    )
}
