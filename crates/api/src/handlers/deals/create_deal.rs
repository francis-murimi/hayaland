use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::deals::dto::CreateDealCommand;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct CreateDealRequest {
    pub title: String,
    pub description: Option<String>,
    pub domain_category_id: Uuid,
    pub consumer_party_id: Uuid,
    pub enhancer_party_id: Uuid,
    pub expected_start_date: Option<time::Date>,
    pub expected_end_date: Option<time::Date>,
    pub timeline: Option<serde_json::Value>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timeout_overrides: Option<serde_json::Value>,
}

pub(crate) fn resolve_actor_party_id(
    req: &HttpRequest,
    _ctx: &AuthContext,
) -> Result<Uuid, ApiError> {
    let header = req
        .headers()
        .get("X-Party-ID")
        .and_then(|h| h.to_str().ok());

    match header {
        Some(value) => value.parse::<Uuid>().map_err(|_| {
            ApiError::Validation("X-Party-ID header must be a valid UUID".to_string())
        }),
        None => Err(ApiError::Validation(
            "X-Party-ID header is required".to_string(),
        )),
    }
}

pub async fn create_deal(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateDealRequest>,
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

    validate_timeout_overrides(&body.timeout_overrides)?;

    let cmd = CreateDealCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        is_admin,
        title: body.title.clone(),
        description: body.description.clone(),
        domain_category_id: body.domain_category_id,
        consumer_party_id: body.consumer_party_id,
        enhancer_party_id: body.enhancer_party_id,
        expected_start_date: body.expected_start_date,
        expected_end_date: body.expected_end_date,
        timeline: body.timeline.clone(),
        latitude: body.latitude,
        longitude: body.longitude,
        timeout_overrides: body.timeout_overrides.clone(),
    };

    let result = state.create_deal.execute(cmd).await?;
    Ok(HttpResponse::Created().json(result))
}

pub(crate) fn validate_timeout_overrides(
    value: &Option<serde_json::Value>,
) -> Result<(), ApiError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value
        .as_object()
        .ok_or_else(|| ApiError::Validation("timeout_overrides must be an object".to_string()))?;
    for (key, val) in object {
        if domain::entities::DealStatus::try_from(key.as_str()).is_err() {
            return Err(ApiError::Validation(format!(
                "timeout_overrides contains invalid status key: {key}"
            )));
        }
        match val {
            serde_json::Value::Null => {}
            serde_json::Value::Number(n) => {
                if n.as_i64().is_none_or(|s| s <= 0) {
                    return Err(ApiError::Validation(format!(
                        "timeout_overrides value for {key} must be a positive integer or null"
                    )));
                }
            }
            _ => {
                return Err(ApiError::Validation(format!(
                    "timeout_overrides value for {key} must be a positive integer or null"
                )));
            }
        }
    }
    Ok(())
}
