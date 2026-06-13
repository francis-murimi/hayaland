use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::deals::dto::SetValueDistributionCommand;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct SetValueDistributionRequest {
    pub total_value: rust_decimal::Decimal,
    pub distribution_model: domain::entities::DistributionModel,
    pub supplier_share_percentage: rust_decimal::Decimal,
    pub enhancer_share_percentage: rust_decimal::Decimal,
    pub platform_fee_percentage: rust_decimal::Decimal,
    pub consumer_cost_percentage: rust_decimal::Decimal,
    pub payment_schedule: Vec<domain::entities::PaymentScheduleEntry>,
}

pub async fn set_value_distribution(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<SetValueDistributionRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;

    let cmd = SetValueDistributionCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        deal_id: path.into_inner(),
        total_value: body.total_value,
        distribution_model: body.distribution_model,
        supplier_share_percentage: body.supplier_share_percentage,
        enhancer_share_percentage: body.enhancer_share_percentage,
        platform_fee_percentage: body.platform_fee_percentage,
        consumer_cost_percentage: body.consumer_cost_percentage,
        payment_schedule: body.payment_schedule.clone(),
    };

    let result = state.set_value_distribution.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_value_distribution(
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
    let actor_party_id = resolve_actor_party_id(&req, &ctx).ok();
    let deal_id = path.into_inner();

    let is_admin = ctx.roles.iter().any(|r| r == "admin");
    let result = state
        .get_value_distribution
        .execute(deal_id, ctx.user_id, actor_party_id, is_admin)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
