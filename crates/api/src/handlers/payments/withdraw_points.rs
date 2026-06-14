use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::payments::dto::WithdrawPointsCommand;
use application::users::token::AuthContext;
use uuid::Uuid;
use validator::Validate;

use crate::errors::ApiError;
use crate::handlers::payments::dto::WithdrawalRequest;
use crate::handlers::payments::is_transaction_admin;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn withdraw_points(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<WithdrawalRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    body.validate()?;

    require_scope_or_admin(&ctx, "payments:write", "admin:transactions")?;

    let cmd = WithdrawPointsCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: path.into_inner(),
        deal_id: body.deal_id,
        amount: body.amount,
        description: body.description.clone(),
        payment_method: body.payment_method.clone(),
        external_reference: body.external_reference.clone(),
        is_admin: is_transaction_admin(&ctx),
    };

    let result = state.withdraw_points.execute(cmd).await?;
    Ok(
        HttpResponse::Created().json(crate::handlers::payments::dto::TransactionResponse::from(
            result,
        )),
    )
}
