use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::agreements::dto::AdminUpdateAgreementCommand;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct AdminUpdateAgreementRequest {
    pub governing_law: Option<String>,
    pub dispute_resolution: Option<String>,
    pub effective_date: Option<time::Date>,
    pub termination_date: Option<time::Date>,
    #[serde(default)]
    pub auto_renew: Option<bool>,
    pub status: Option<domain::entities::AgreementStatus>,
    pub reason: Option<String>,
}

pub async fn admin_update_agreement(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<AdminUpdateAgreementRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    if !ctx.has_scope("admin:deals") && !ctx.has_scope("admin:*") {
        return Err(ApiError::Forbidden);
    }

    let cmd = AdminUpdateAgreementCommand {
        admin_user_id: ctx.user_id,
        deal_id: path.into_inner(),
        governing_law: body.governing_law.clone(),
        dispute_resolution: body.dispute_resolution.clone(),
        effective_date: body.effective_date,
        termination_date: body.termination_date,
        auto_renew: body.auto_renew,
        status: body.status,
        reason: body.reason.clone(),
    };

    let result = state.admin_update_agreement.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}
