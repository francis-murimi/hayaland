use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::milestones::dto::{
    CreateMilestoneCommand, GetDealProgressQuery, ListMilestonesQuery, MilestoneActionCommand,
    UpdateMilestoneCommand,
};
use application::users::token::AuthContext;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::errors::ApiError;

mod date_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    #[allow(dead_code)]
    pub fn serialize<S: Serializer>(date: &Option<time::Date>, s: S) -> Result<S::Ok, S::Error> {
        match date {
            Some(d) => s.serialize_some(&d.to_string()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<time::Date>, D::Error> {
        let s: Option<String> = Option::deserialize(d)?;
        match s {
            Some(v) => time::Date::parse(&v, &time::format_description::well_known::Iso8601::DATE)
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::AppState;

fn is_milestone_admin(ctx: &AuthContext) -> bool {
    ctx.has_role("admin") || ctx.has_scope("admin:milestones") || ctx.has_scope("admin:*")
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateMilestoneRequest {
    pub milestone_name: String,
    pub description: Option<String>,
    pub assigned_to_party_id: Uuid,
    pub verified_by_party_id: Uuid,
    #[serde(with = "crate::handlers::milestones::date_serde")]
    pub due_date: Option<time::Date>,
    pub completion_criteria: String,
    pub payment_trigger_amount: Option<Decimal>,
    pub display_order: i32,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct UpdateMilestoneRequest {
    pub milestone_name: Option<String>,
    pub description: Option<String>,
    pub assigned_to_party_id: Option<Uuid>,
    pub verified_by_party_id: Option<Uuid>,
    #[serde(with = "crate::handlers::milestones::date_serde", default)]
    pub due_date: Option<time::Date>,
    pub completion_criteria: Option<String>,
    pub payment_trigger_amount: Option<Decimal>,
    pub display_order: Option<i32>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct MilestoneActionRequest {
    pub comment: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ListMilestonesQueryParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn create_milestone(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<CreateMilestoneRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;

    let result = state
        .create_milestone
        .execute(CreateMilestoneCommand {
            actor_user_id: ctx.user_id,
            actor_party_id,
            is_admin: is_milestone_admin(&ctx),
            deal_id: path.into_inner(),
            milestone_name: body.milestone_name.clone(),
            description: body.description.clone(),
            assigned_to_party_id: body.assigned_to_party_id,
            verified_by_party_id: body.verified_by_party_id,
            due_date: body.due_date,
            completion_criteria: body.completion_criteria.clone(),
            payment_trigger_amount: body.payment_trigger_amount,
            display_order: body.display_order,
        })
        .await?;

    Ok(HttpResponse::Created().json(MilestoneResponse::from(result)))
}

pub async fn list_milestones(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<ListMilestonesQueryParams>,
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
        .list_milestones
        .execute(ListMilestonesQuery {
            actor_user_id: ctx.user_id,
            actor_party_id,
            is_admin: is_milestone_admin(&ctx),
            deal_id: path.into_inner(),
            limit: Some(per_page),
            offset: Some(offset),
        })
        .await?;

    Ok(HttpResponse::Ok().json(MilestonesResponse::from(result)))
}

pub async fn get_deal_progress(
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
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;

    let result = state
        .get_deal_progress
        .execute(GetDealProgressQuery {
            actor_user_id: ctx.user_id,
            actor_party_id,
            is_admin: is_milestone_admin(&ctx),
            deal_id: path.into_inner(),
        })
        .await?;

    Ok(HttpResponse::Ok().json(DealProgressResponse::from(result)))
}

pub async fn update_milestone(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<UpdateMilestoneRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let (_deal_id, milestone_id) = path.into_inner();

    let result = state
        .update_milestone
        .execute(UpdateMilestoneCommand {
            actor_user_id: ctx.user_id,
            actor_party_id,
            is_admin: is_milestone_admin(&ctx),
            milestone_id,
            milestone_name: body.milestone_name.clone(),
            description: body.description.clone(),
            assigned_to_party_id: body.assigned_to_party_id,
            verified_by_party_id: body.verified_by_party_id,
            due_date: body.due_date,
            completion_criteria: body.completion_criteria.clone(),
            payment_trigger_amount: body.payment_trigger_amount,
            display_order: body.display_order,
        })
        .await?;

    Ok(HttpResponse::Ok().json(MilestoneResponse::from(result)))
}

pub async fn delete_milestone(
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
    let (_deal_id, milestone_id) = path.into_inner();

    state
        .delete_milestone
        .execute(MilestoneActionCommand {
            actor_user_id: ctx.user_id,
            actor_party_id,
            is_admin: is_milestone_admin(&ctx),
            milestone_id,
            comment: None,
        })
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

pub async fn start_milestone(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    _body: web::Json<MilestoneActionRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let (_deal_id, milestone_id) = path.into_inner();

    let result = state
        .start_milestone
        .execute(MilestoneActionCommand {
            actor_user_id: ctx.user_id,
            actor_party_id,
            is_admin: is_milestone_admin(&ctx),
            milestone_id,
            comment: None,
        })
        .await?;

    Ok(HttpResponse::Ok().json(MilestoneResponse::from(result)))
}

pub async fn complete_milestone(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<MilestoneActionRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let (_deal_id, milestone_id) = path.into_inner();

    let result = state
        .complete_milestone
        .execute(MilestoneActionCommand {
            actor_user_id: ctx.user_id,
            actor_party_id,
            is_admin: is_milestone_admin(&ctx),
            milestone_id,
            comment: body.comment.clone(),
        })
        .await?;

    Ok(HttpResponse::Ok().json(MilestoneResponse::from(result)))
}

pub async fn verify_milestone(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<MilestoneActionRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;
    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let (_deal_id, milestone_id) = path.into_inner();

    let result = state
        .verify_milestone
        .execute(MilestoneActionCommand {
            actor_user_id: ctx.user_id,
            actor_party_id,
            is_admin: is_milestone_admin(&ctx),
            milestone_id,
            comment: body.comment.clone(),
        })
        .await?;

    Ok(HttpResponse::Ok().json(MilestoneWithTransactionResponse::from(result)))
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MilestoneResponse {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub milestone_name: String,
    pub description: Option<String>,
    pub assigned_to_party_id: Uuid,
    pub verified_by_party_id: Uuid,
    pub due_date: Option<time::Date>,
    pub completion_criteria: String,
    pub milestone_status: String,
    pub completion_percentage: Decimal,
    pub payment_trigger_amount: Option<Decimal>,
    pub completed_at: Option<time::OffsetDateTime>,
    pub display_order: i32,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

impl From<application::milestones::dto::MilestoneResult> for MilestoneResponse {
    fn from(result: application::milestones::dto::MilestoneResult) -> Self {
        Self {
            id: result.id,
            deal_id: result.deal_id,
            milestone_name: result.milestone_name,
            description: result.description,
            assigned_to_party_id: result.assigned_to_party_id,
            verified_by_party_id: result.verified_by_party_id,
            due_date: result.due_date,
            completion_criteria: result.completion_criteria,
            milestone_status: result.milestone_status,
            completion_percentage: result.completion_percentage,
            payment_trigger_amount: result.payment_trigger_amount,
            completed_at: result.completed_at,
            display_order: result.display_order,
            created_at: result.created_at,
            updated_at: result.updated_at,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MilestoneWithTransactionResponse {
    #[serde(flatten)]
    pub milestone: MilestoneResponse,
    pub triggered_transaction_id: Option<Uuid>,
}

impl From<application::milestones::dto::MilestoneWithTransactionResult>
    for MilestoneWithTransactionResponse
{
    fn from(result: application::milestones::dto::MilestoneWithTransactionResult) -> Self {
        Self {
            milestone: result.milestone.into(),
            triggered_transaction_id: result.triggered_transaction_id,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MilestonesResponse {
    pub milestones: Vec<MilestoneResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

impl From<application::milestones::dto::ListMilestonesResult> for MilestonesResponse {
    fn from(result: application::milestones::dto::ListMilestonesResult) -> Self {
        Self {
            milestones: result.milestones.into_iter().map(Into::into).collect(),
            total: result.total,
            page: (result.offset / result.limit) + 1,
            per_page: result.limit,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DealProgressResponse {
    pub deal_id: Uuid,
    pub total_milestones: i64,
    pub verified_milestones: i64,
    pub completed_milestones: i64,
    pub in_progress_milestones: i64,
    pub missed_milestones: i64,
    pub overall_completion_percentage: Decimal,
    pub currency: String,
}

impl From<application::milestones::dto::DealProgressResult> for DealProgressResponse {
    fn from(result: application::milestones::dto::DealProgressResult) -> Self {
        Self {
            deal_id: result.deal_id,
            total_milestones: result.total_milestones,
            verified_milestones: result.verified_milestones,
            completed_milestones: result.completed_milestones,
            in_progress_milestones: result.in_progress_milestones,
            missed_milestones: result.missed_milestones,
            overall_completion_percentage: result.overall_completion_percentage,
            currency: result.currency.as_str().to_string(),
        }
    }
}
