use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::deals::dto::UpdateDealCommand;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct UpdateDealRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub domain_category_id: Option<Uuid>,
    pub expected_start_date: Option<time::Date>,
    pub expected_end_date: Option<time::Date>,
    pub timeline: Option<serde_json::Value>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timeout_overrides: Option<serde_json::Value>,
}

pub async fn update_deal(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<UpdateDealRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "deals:write", "admin:deals")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let is_admin = ctx.has_scope("admin:deals") || ctx.has_scope("admin:*");

    crate::handlers::deals::create_deal::validate_timeout_overrides(&body.timeout_overrides)?;

    let cmd = UpdateDealCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        is_admin,
        title: body.title.clone(),
        description: body.description.clone(),
        domain_category_id: body.domain_category_id,
        expected_start_date: body.expected_start_date,
        expected_end_date: body.expected_end_date,
        timeline: body.timeline.clone(),
        latitude: body.latitude,
        longitude: body.longitude,
        timeout_overrides: body.timeout_overrides.clone(),
    };

    let result = state.update_deal.execute(path.into_inner(), cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}
