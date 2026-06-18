use crate::dto::catalog::{BindCatalogItemRequest, UpdateDealResourceRequest};
use crate::errors::ApiError;
use crate::handlers::catalog::{is_catalog_admin, require_ctx};
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::catalog::dto::{BindCatalogItemToDealCommand, UpdateResourceCommand};
use uuid::Uuid;
use validator::Validate;

pub async fn bind_resource(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<BindCatalogItemRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    bind_item(state, path.into_inner(), "RESOURCE", body.into_inner(), req).await
}

pub async fn bind_need(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<BindCatalogItemRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    bind_item(state, path.into_inner(), "NEED", body.into_inner(), req).await
}

pub async fn bind_enhancement(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<BindCatalogItemRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    bind_item(
        state,
        path.into_inner(),
        "ENHANCEMENT",
        body.into_inner(),
        req,
    )
    .await
}

async fn bind_item(
    state: web::Data<AppState>,
    deal_id: Uuid,
    item_type: &str,
    body: BindCatalogItemRequest,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = require_ctx(&req)?;
    require_scope_or_admin(&ctx, "catalog:write", "admin:catalog")?;
    let party_id = crate::handlers::catalog::actor_party_id(&req)?;

    let cmd = BindCatalogItemToDealCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: party_id,
        is_admin: is_catalog_admin(&ctx),
        item_type: item_type.to_string(),
        item_id: body.item_id,
        deal_id,
        overrides: body.overrides,
    };

    let result = state.bind_catalog_item_to_deal.execute(cmd).await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn list_deal_resources(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let result = state
        .list_deal_catalog_items
        .list_resources(path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn list_deal_needs(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let result = state
        .list_deal_catalog_items
        .list_needs(path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn list_deal_enhancements(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let result = state
        .list_deal_catalog_items
        .list_enhancements(path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn update_deal_resource(
    state: web::Data<AppState>,
    _path: web::Path<Uuid>,
    body: web::Json<UpdateDealResourceRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = require_ctx(&req)?;
    require_scope_or_admin(&ctx, "catalog:write", "admin:catalog")?;
    let party_id = crate::handlers::catalog::actor_party_id(&req)?;

    let update = &body.update;
    let cmd = UpdateResourceCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: party_id,
        is_admin: is_catalog_admin(&ctx),
        resource_type_id: update.resource_type_id,
        resource_name: update.resource_name.clone(),
        description: update.description.clone(),
        quantity: update.quantity,
        quantity_unit: update.quantity_unit.clone(),
        condition: update.condition,
        latitude: update.latitude,
        longitude: update.longitude,
        location_address: update.location_address.clone(),
        availability_start: update.availability_start,
        availability_end: update.availability_end,
        document_urls: update.document_urls.clone(),
        opportunity_cost: update.opportunity_cost,
        metadata: update.metadata.clone(),
        is_active: update.is_active,
    };

    let result = state.update_resource.execute(body.item_id, cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}
