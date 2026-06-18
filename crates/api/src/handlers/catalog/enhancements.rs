use crate::dto::catalog::{
    CatalogSearchQueryParams, CreateEnhancementRequest, EnhancementResponse,
    UpdateEnhancementRequest,
};
use crate::errors::ApiError;
use crate::handlers::catalog::{catalog_actor, is_catalog_admin, require_ctx};
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;
use actix_web::{http::header::LOCATION, web, HttpRequest, HttpResponse};
use application::catalog::dto::{
    CreateEnhancementCommand, DeleteCatalogItemCommand, UpdateEnhancementCommand,
};
use application::catalog::EnhancementView;
use validator::Validate;

pub async fn create_enhancement(
    state: web::Data<AppState>,
    body: web::Json<CreateEnhancementRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = require_ctx(&req)?;
    require_scope_or_admin(&ctx, "catalog:write", "admin:catalog")?;
    let party_id = crate::handlers::catalog::actor_party_id(&req)?;

    let cmd = CreateEnhancementCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: party_id,
        is_admin: is_catalog_admin(&ctx),
        enhancement_type_id: body.enhancement_type_id,
        enhancement_name: body.enhancement_name.clone(),
        description: body.description.clone(),
        input_quantity: body.input_quantity,
        quantity_unit: body.quantity_unit.clone(),
        estimated_input_cost: body.estimated_input_cost,
        service_duration_hours: body.service_duration_hours,
        estimated_completion_days: body.estimated_completion_days,
        deliverables: body.deliverables.clone(),
        prerequisites: body.prerequisites.clone(),
        skills: body.skills.clone(),
        certifications: body.certifications.clone(),
        equipment: body.equipment.clone(),
        pricing: body.pricing.clone(),
        availability: body.availability.clone(),
        service_area: body.service_area.clone(),
        metadata: body.metadata.clone(),
    };

    let result = state.create_enhancement.execute(cmd).await?;
    let location = format!("/api/v1/enhancements/{}", result.id);

    Ok(HttpResponse::Created()
        .insert_header((LOCATION, location))
        .json(EnhancementResponse::from(result)))
}

pub async fn list_enhancements(
    state: web::Data<AppState>,
    query: web::Query<CatalogSearchQueryParams>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let params = query.into_inner();
    params.validate().map_err(ApiError::from)?;

    let (actor_party_id, is_admin) = catalog_actor(&req)?;
    let app_query = application::catalog::dto::CatalogSearchQuery::from(params);
    let result = state
        .list_enhancements
        .execute(app_query, actor_party_id, is_admin)
        .await?;

    Ok(HttpResponse::Ok().json(result))
}

pub async fn search_enhancements(
    state: web::Data<AppState>,
    query: web::Query<CatalogSearchQueryParams>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    list_enhancements(state, query, req).await
}

pub async fn get_enhancement(
    state: web::Data<AppState>,
    path: web::Path<uuid::Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let (actor_party_id, is_admin) = catalog_actor(&req)?;
    let view = state
        .get_enhancement
        .execute(path.into_inner(), actor_party_id, is_admin)
        .await?;

    let response = match view {
        EnhancementView::Owner(e) => EnhancementResponse::Owner(e),
        EnhancementView::Public(e) => EnhancementResponse::Public(e),
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn update_enhancement(
    state: web::Data<AppState>,
    path: web::Path<uuid::Uuid>,
    body: web::Json<UpdateEnhancementRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = require_ctx(&req)?;
    require_scope_or_admin(&ctx, "catalog:write", "admin:catalog")?;
    let party_id = crate::handlers::catalog::actor_party_id(&req)?;

    let cmd = UpdateEnhancementCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: party_id,
        is_admin: is_catalog_admin(&ctx),
        enhancement_type_id: body.enhancement_type_id,
        enhancement_name: body.enhancement_name.clone(),
        description: body.description.clone(),
        input_quantity: body.input_quantity,
        quantity_unit: body.quantity_unit.clone(),
        estimated_input_cost: body.estimated_input_cost,
        service_duration_hours: body.service_duration_hours,
        estimated_completion_days: body.estimated_completion_days,
        deliverables: body.deliverables.clone(),
        prerequisites: body.prerequisites.clone(),
        skills: body.skills.clone(),
        certifications: body.certifications.clone(),
        equipment: body.equipment.clone(),
        pricing: body.pricing.clone(),
        availability: body.availability.clone(),
        service_area: body.service_area.clone(),
        metadata: body.metadata.clone(),
        is_active: body.is_active,
    };

    let result = state
        .update_enhancement
        .execute(path.into_inner(), cmd)
        .await?;
    Ok(HttpResponse::Ok().json(EnhancementResponse::from(result)))
}

pub async fn delete_enhancement(
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

    state
        .delete_enhancement
        .execute(path.into_inner(), cmd)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
