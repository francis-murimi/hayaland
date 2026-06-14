use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::payments::dto::ApproveTransactionCommand;
use application::users::token::AuthContext;
use domain::entities::ApprovalDecision;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::AppState;

#[derive(Debug, serde::Deserialize, Default)]
pub struct ApproveTransactionRequest {
    pub comment: Option<String>,
}

pub async fn approve_transaction(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<ApproveTransactionRequest>,
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
        .approve_transaction
        .execute(ApproveTransactionCommand {
            actor_user_id: ctx.user_id,
            actor_party_id,
            transaction_id: path.into_inner(),
            decision: ApprovalDecision::Approved,
            comment: body.comment.clone(),
        })
        .await?;

    Ok(
        HttpResponse::Ok().json(crate::handlers::payments::dto::TransactionResponse::from(
            result,
        )),
    )
}
