use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::agreements::dto::SignAgreementCommand;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct SignAgreementRequest {
    #[serde(default)]
    pub signature_type: Option<domain::entities::SignatureType>,
}

pub async fn sign_agreement(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<SignAgreementRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;

    let ip_address = req.peer_addr().map(|addr| addr.ip().to_string());

    let cmd = SignAgreementCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        deal_id: path.into_inner(),
        signature_type: body.signature_type.unwrap_or_default(),
        ip_address,
    };

    let result = state.sign_agreement.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}
