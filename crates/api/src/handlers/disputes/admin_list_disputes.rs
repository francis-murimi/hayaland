use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::disputes::dto::AdminDisputeListQuery;
use application::users::token::AuthContext;

use crate::errors::ApiError;
use crate::handlers::disputes::dto::DisputesResponse;
use crate::middleware::auth::require_scope;
use crate::AppState;

pub async fn admin_list_disputes(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<AdminDisputeListQuery>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope(&ctx, "admin:disputes")?;

    let result = state
        .list_admin_disputes
        .execute(query.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(DisputesResponse::from(result)))
}
