use crate::dto::catalog::{
    CatalogSearchQueryParams, CreateResourceRequest, ResourceResponse, UpdateResourceRequest,
};
use crate::errors::ApiError;
use crate::handlers::catalog::{catalog_actor, require_ctx};
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;
use actix_web::{http::header::LOCATION, web, HttpRequest, HttpResponse};
use application::catalog::dto::{
    CreateResourceCommand, DeleteCatalogItemCommand, UpdateResourceCommand,
};
use application::catalog::ResourceView;
use validator::Validate;

pub async fn create_resource(
    state: web::Data<AppState>,
    body: web::Json<CreateResourceRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = require_ctx(&req)?;
    require_scope_or_admin(&ctx, "catalog:write", "admin:catalog")?;
    let party_id = crate::handlers::catalog::actor_party_id(&req)?;

    let cmd = CreateResourceCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: party_id,
        is_admin: crate::handlers::catalog::is_catalog_admin(&ctx),
        resource_type_id: body.resource_type_id,
        resource_name: body.resource_name.clone(),
        description: body.description.clone(),
        quantity: body.quantity,
        quantity_unit: body.quantity_unit.clone(),
        condition: body.condition,
        latitude: body.latitude,
        longitude: body.longitude,
        location_address: body.location_address.clone(),
        availability_start: body.availability_start,
        availability_end: body.availability_end,
        document_urls: body.document_urls.clone(),
        opportunity_cost: body.opportunity_cost,
        metadata: body.metadata.clone(),
    };

    let result = state.create_resource.execute(cmd).await?;
    let location = format!("/api/v1/resources/{}", result.id);

    Ok(HttpResponse::Created()
        .insert_header((LOCATION, location))
        .json(ResourceResponse::from(result)))
}

pub async fn list_resources(
    state: web::Data<AppState>,
    query: web::Query<CatalogSearchQueryParams>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let params = query.into_inner();
    params.validate().map_err(ApiError::from)?;

    let (actor_party_id, is_admin) = catalog_actor(&req)?;
    let app_query = application::catalog::dto::CatalogSearchQuery::from(params);
    let result = state
        .list_resources
        .execute(app_query, actor_party_id, is_admin)
        .await?;

    Ok(HttpResponse::Ok().json(result))
}

pub async fn search_resources(
    state: web::Data<AppState>,
    query: web::Query<CatalogSearchQueryParams>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    list_resources(state, query, req).await
}

pub async fn get_resource(
    state: web::Data<AppState>,
    path: web::Path<uuid::Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let (actor_party_id, is_admin) = catalog_actor(&req)?;
    let view = state
        .get_resource
        .execute(path.into_inner(), actor_party_id, is_admin)
        .await?;

    let response = match view {
        ResourceView::Owner(r) => ResourceResponse::Owner(r),
        ResourceView::Public(r) => ResourceResponse::Public(r),
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn update_resource(
    state: web::Data<AppState>,
    path: web::Path<uuid::Uuid>,
    body: web::Json<UpdateResourceRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = require_ctx(&req)?;
    require_scope_or_admin(&ctx, "catalog:write", "admin:catalog")?;
    let party_id = crate::handlers::catalog::actor_party_id(&req)?;

    let cmd = UpdateResourceCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: party_id,
        is_admin: crate::handlers::catalog::is_catalog_admin(&ctx),
        resource_type_id: body.resource_type_id,
        resource_name: body.resource_name.clone(),
        description: body.description.clone(),
        quantity: body.quantity,
        quantity_unit: body.quantity_unit.clone(),
        condition: body.condition,
        latitude: body.latitude,
        longitude: body.longitude,
        location_address: body.location_address.clone(),
        availability_start: body.availability_start,
        availability_end: body.availability_end,
        document_urls: body.document_urls.clone(),
        opportunity_cost: body.opportunity_cost,
        metadata: body.metadata.clone(),
        is_active: body.is_active,
    };

    let result = state
        .update_resource
        .execute(path.into_inner(), cmd)
        .await?;
    Ok(HttpResponse::Ok().json(ResourceResponse::from(result)))
}

pub async fn delete_resource(
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
        is_admin: crate::handlers::catalog::is_catalog_admin(&ctx),
    };

    state
        .delete_resource
        .execute(path.into_inner(), cmd)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
