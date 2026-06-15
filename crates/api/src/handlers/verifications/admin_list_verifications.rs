use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use application::verifications::dto::AdminVerificationListQuery as AppQuery;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::verifications::dto::VerificationsResponse;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct Query {
    pub status: Option<String>,
    #[serde(rename = "verificationType")]
    pub verification_type: Option<String>,
    #[serde(rename = "partyId")]
    pub party_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn admin_list_verifications(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<Query>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "admin:verifications", "admin:verifications")?;

    let app_query = AppQuery {
        status: query.status.clone(),
        verification_type: query.verification_type.clone(),
        party_id: query.party_id,
        limit: query.limit.unwrap_or(20).clamp(1, 100),
        offset: query.offset.unwrap_or(0).max(0),
    };

    let result = state.list_admin_verifications.execute(app_query).await?;
    Ok(HttpResponse::Ok().json(VerificationsResponse::from(result)))
}
