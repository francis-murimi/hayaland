use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::matching::dto::{AdminDeleteMatchesCommand, AdminUpdateMatchCommand};
use application::users::token::AuthContext;
use domain::repositories::MatchFilters;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::matches::services;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct AdminUpdateMatchRequest {
    pub new_status: domain::entities::MatchStatus,
    pub reason: Option<String>,
}

pub async fn list_all_matches(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "matches:read", "admin:matches")?;

    let use_case = services::admin_match_controls(state.db_pool.clone());
    let result = use_case.list_all(&MatchFilters::default()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn update_match_status(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<AdminUpdateMatchRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "matches:write", "admin:matches")?;

    let cmd = AdminUpdateMatchCommand {
        admin_user_id: ctx.user_id,
        match_suggestion_id: path.into_inner(),
        new_status: body.new_status,
        reason: body.reason.clone(),
    };

    let use_case = services::admin_match_controls(state.db_pool.clone());
    use_case.update_status(cmd).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn delete_match_suggestions_for_party(
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

    require_scope_or_admin(&ctx, "matches:write", "admin:matches")?;

    let cmd = AdminDeleteMatchesCommand {
        admin_user_id: ctx.user_id,
        party_id: path.into_inner(),
        status: None,
    };

    let use_case = services::admin_match_controls(state.db_pool.clone());
    let deleted = use_case.delete_for_party(cmd).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "deleted": deleted })))
}

pub async fn delete_all_match_suggestions(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "matches:write", "admin:matches")?;

    let use_case = services::admin_match_controls(state.db_pool.clone());
    let deleted = use_case.delete_all(ctx.user_id).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "deleted": deleted })))
}

pub async fn get_platform_status_counts(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "matches:read", "admin:matches")?;

    let use_case = services::admin_match_controls(state.db_pool.clone());
    let result = use_case.count_platform().await?;
    Ok(HttpResponse::Ok().json(result))
}
