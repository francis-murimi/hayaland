use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::payments::is_transaction_admin;
use crate::AppState;

pub async fn get_transaction(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;

    let result = state
        .get_transaction
        .execute(
            ctx.user_id,
            actor_party_id,
            path.into_inner(),
            is_transaction_admin(&ctx),
        )
        .await?;

    Ok(HttpResponse::Ok()
        .json(crate::handlers::payments::dto::TransactionWithApprovalsResponse::from(result)))
}
