use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::deals::dto::ExecuteTransitionCommand;
use application::users::token::AuthContext;
use domain::entities::DealStatus;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct ExecuteTransitionRequest {
    pub new_status: DealStatus,
    pub reason: Option<String>,
    #[serde(default)]
    pub acknowledge_warnings: bool,
}

pub async fn execute_transition(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<ExecuteTransitionRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;

    let cmd = ExecuteTransitionCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        new_status: body.new_status,
        reason: body.reason.clone(),
        acknowledge_warnings: body.acknowledge_warnings,
    };

    let result = state
        .execute_transition
        .execute(path.into_inner(), cmd)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
