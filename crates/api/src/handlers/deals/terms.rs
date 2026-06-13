use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::deals::dto::{CounterTermCommand, ProposeTermCommand, TermActionCommand};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct ProposeTermRequest {
    pub term_type: domain::entities::TermType,
    pub term_name: String,
    pub description: String,
    #[serde(default)]
    pub is_mandatory: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct CounterTermRequest {
    pub description: String,
}

pub async fn propose_term(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<ProposeTermRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;

    let cmd = ProposeTermCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        deal_id: path.into_inner(),
        term_type: body.term_type,
        term_name: body.term_name.clone(),
        description: body.description.clone(),
        is_mandatory: body.is_mandatory,
    };

    let result = state.propose_term.execute(cmd).await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn counter_term(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<CounterTermRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let (deal_id, term_id) = path.into_inner();

    let cmd = CounterTermCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        deal_id,
        term_id,
        description: body.description.clone(),
    };

    let result = state.counter_term.execute(cmd).await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn accept_term(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let (deal_id, term_id) = path.into_inner();

    let cmd = TermActionCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        deal_id,
        term_id,
    };

    let result = state.accept_term.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn reject_term(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let (deal_id, term_id) = path.into_inner();

    let cmd = TermActionCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        deal_id,
        term_id,
    };

    let result = state.reject_term.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn withdraw_term(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let (deal_id, term_id) = path.into_inner();

    let cmd = TermActionCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        deal_id,
        term_id,
    };

    let result = state.withdraw_term.execute(cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn list_terms(
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
    let actor_party_id = resolve_actor_party_id(&req, &ctx).ok();
    let deal_id = path.into_inner();

    let is_admin = ctx.roles.iter().any(|r| r == "admin");
    let result = state
        .list_terms
        .execute(deal_id, ctx.user_id, actor_party_id, is_admin)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
