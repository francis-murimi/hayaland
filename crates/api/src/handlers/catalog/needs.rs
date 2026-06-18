use crate::dto::catalog::{
    CatalogSearchQueryParams, CreateNeedRequest, NeedResponse, UpdateNeedRequest,
};
use crate::errors::ApiError;
use crate::handlers::catalog::{catalog_actor, is_catalog_admin, require_ctx};
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;
use actix_web::{http::header::LOCATION, web, HttpRequest, HttpResponse};
use application::catalog::dto::{CreateNeedCommand, DeleteCatalogItemCommand, UpdateNeedCommand};
use application::catalog::NeedView;
use validator::Validate;

pub async fn create_need(
    state: web::Data<AppState>,
    body: web::Json<CreateNeedRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = require_ctx(&req)?;
    require_scope_or_admin(&ctx, "catalog:write", "admin:catalog")?;
    let party_id = crate::handlers::catalog::actor_party_id(&req)?;

    let cmd = CreateNeedCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: party_id,
        is_admin: is_catalog_admin(&ctx),
        need_category_id: body.need_category_id,
        need_description: body.need_description.clone(),
        required_quantity: body.required_quantity,
        quantity_unit: body.quantity_unit.clone(),
        quality_requirements: body.quality_requirements.clone(),
        required_by_date: body.required_by_date,
        max_budget: body.max_budget,
        budget_currency: body.budget_currency.clone(),
        estimated_fulfillment_value: body.estimated_fulfillment_value,
        acceptable_variants: body.acceptable_variants.clone(),
        priority: body.priority,
        latitude: body.latitude,
        longitude: body.longitude,
        location_address: body.location_address.clone(),
        delivery_preferences: body.delivery_preferences.clone(),
        metadata: body.metadata.clone(),
    };

    let result = state.create_need.execute(cmd).await?;
    let location = format!("/api/v1/needs/{}", result.id);

    Ok(HttpResponse::Created()
        .insert_header((LOCATION, location))
        .json(NeedResponse::from(result)))
}

pub async fn list_needs(
    state: web::Data<AppState>,
    query: web::Query<CatalogSearchQueryParams>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let params = query.into_inner();
    params.validate().map_err(ApiError::from)?;

    let (actor_party_id, is_admin) = catalog_actor(&req)?;
    let app_query = application::catalog::dto::CatalogSearchQuery::from(params);
    let result = state
        .list_needs
        .execute(app_query, actor_party_id, is_admin)
        .await?;

    Ok(HttpResponse::Ok().json(result))
}

pub async fn search_needs(
    state: web::Data<AppState>,
    query: web::Query<CatalogSearchQueryParams>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    list_needs(state, query, req).await
}

pub async fn get_need(
    state: web::Data<AppState>,
    path: web::Path<uuid::Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let (actor_party_id, is_admin) = catalog_actor(&req)?;
    let view = state
        .get_need
        .execute(path.into_inner(), actor_party_id, is_admin)
        .await?;

    let response = match view {
        NeedView::Owner(n) => NeedResponse::Owner(n),
        NeedView::Public(n) => NeedResponse::Public(n),
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn update_need(
    state: web::Data<AppState>,
    path: web::Path<uuid::Uuid>,
    body: web::Json<UpdateNeedRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = require_ctx(&req)?;
    require_scope_or_admin(&ctx, "catalog:write", "admin:catalog")?;
    let party_id = crate::handlers::catalog::actor_party_id(&req)?;

    let cmd = UpdateNeedCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: party_id,
        is_admin: is_catalog_admin(&ctx),
        need_category_id: body.need_category_id,
        need_description: body.need_description.clone(),
        required_quantity: body.required_quantity,
        quantity_unit: body.quantity_unit.clone(),
        quality_requirements: body.quality_requirements.clone(),
        required_by_date: body.required_by_date,
        max_budget: body.max_budget,
        budget_currency: body.budget_currency.clone(),
        estimated_fulfillment_value: body.estimated_fulfillment_value,
        acceptable_variants: body.acceptable_variants.clone(),
        priority: body.priority,
        latitude: body.latitude,
        longitude: body.longitude,
        location_address: body.location_address.clone(),
        delivery_preferences: body.delivery_preferences.clone(),
        metadata: body.metadata.clone(),
        is_active: body.is_active,
    };

    let result = state.update_need.execute(path.into_inner(), cmd).await?;
    Ok(HttpResponse::Ok().json(NeedResponse::from(result)))
}

pub async fn delete_need(
    state: web::Data<AppState>,
    path: web::Path<uuid::Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = require_ctx(&req)?;
    require_scope_or_admin(&ctx, "catalog:write", "admin:catalog")?;
    let party_id = crate::handlers::catalog::actor_party_id(&req)?;

    let cmd = DeleteCatalogItemCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: party_id,
        is_admin: is_catalog_admin(&ctx),
    };

    state.delete_need.execute(path.into_inner(), cmd).await?;
    Ok(HttpResponse::NoContent().finish())
}
