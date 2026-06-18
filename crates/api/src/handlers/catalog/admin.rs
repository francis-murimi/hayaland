use crate::dto::catalog::{AdminCatalogItemResponse, AdminUpdateCatalogFlagsRequest};
use crate::errors::ApiError;
use crate::handlers::catalog::{is_catalog_admin, require_ctx};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::catalog::dto::AdminUpdateFlagsCommand;
use uuid::Uuid;
use validator::Validate;

pub async fn update_resource_flags(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<AdminUpdateCatalogFlagsRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    update_flags(state, path.into_inner(), "RESOURCE", body.into_inner(), req).await
}

pub async fn update_need_flags(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<AdminUpdateCatalogFlagsRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    update_flags(state, path.into_inner(), "NEED", body.into_inner(), req).await
}

pub async fn update_enhancement_flags(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<AdminUpdateCatalogFlagsRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    update_flags(
        state,
        path.into_inner(),
        "ENHANCEMENT",
        body.into_inner(),
        req,
    )
    .await
}

async fn update_flags(
    state: web::Data<AppState>,
    id: Uuid,
    item_type: &str,
    body: AdminUpdateCatalogFlagsRequest,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = require_ctx(&req)?;
    if !is_catalog_admin(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let party_id =
        crate::handlers::catalog::optional_actor_party_id(&req).unwrap_or_else(Uuid::nil);

    let cmd = AdminUpdateFlagsCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: party_id,
        is_admin: true,
        platform_hidden: body.platform_hidden,
        platform_featured: body.platform_featured,
        admin_notes: body.admin_notes,
    };

    let result = state
        .admin_update_catalog_flags
        .execute(item_type, id, cmd)
        .await?;
    let response = AdminCatalogItemResponse::from(result);
    Ok(HttpResponse::Ok().json(response))
}
