use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::deals::dto::CreateDealCommand;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
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

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;

    let cmd = CreateDealCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
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
    };

    let result = state.create_deal.execute(cmd).await?;
    Ok(HttpResponse::Created().json(result))
}
