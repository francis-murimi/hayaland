use crate::handlers::payments::dto::ListTransactionsQuery;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::AppState;

pub async fn list_deal_transactions(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    query: web::Query<ListTransactionsQuery>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    let (party_id, deal_id) = path.into_inner();
    let is_admin = ctx.has_scope("admin:transactions") || ctx.has_scope("admin:*");
    let result = state
        .list_deal_transactions
        .execute(
            ctx.user_id,
            party_id,
            deal_id,
            query.into_inner().into(),
            is_admin,
        )
        .await?;
    Ok(
        HttpResponse::Ok().json(crate::handlers::payments::dto::TransactionsResponse::from(
            result,
        )),
    )
}
