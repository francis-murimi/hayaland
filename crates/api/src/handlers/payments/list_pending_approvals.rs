use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::payments::dto::ListPendingApprovalsQuery;
use application::users::token::AuthContext;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct PendingApprovalsQueryParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn list_pending_approvals(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<PendingApprovalsQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;

    const DEFAULT_PER_PAGE: i64 = 20;
    const MAX_PER_PAGE: i64 = 100;
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let result = state
        .list_pending_approvals
        .execute(ListPendingApprovalsQuery {
            actor_user_id: ctx.user_id,
            actor_party_id,
            limit: Some(per_page),
            offset: Some(offset),
        })
        .await?;

    Ok(HttpResponse::Ok()
        .json(crate::handlers::payments::dto::PendingApprovalsResponse::from(result)))
}
