use crate::dto::catalog::ContactCatalogOwnerRequest;
use crate::errors::ApiError;
use crate::handlers::catalog::{is_catalog_admin, require_ctx};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::catalog::dto::ContactCatalogOwnerCommand;
use uuid::Uuid;
use validator::Validate;

pub async fn contact_resource_owner(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<ContactCatalogOwnerRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    contact_owner(state, path.into_inner(), "RESOURCE", body.into_inner(), req).await
}

pub async fn contact_need_owner(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<ContactCatalogOwnerRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    contact_owner(state, path.into_inner(), "NEED", body.into_inner(), req).await
}

pub async fn contact_enhancement_owner(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<ContactCatalogOwnerRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    contact_owner(
        state,
        path.into_inner(),
        "ENHANCEMENT",
        body.into_inner(),
        req,
    )
    .await
}

async fn contact_owner(
    state: web::Data<AppState>,
    item_id: Uuid,
    item_type: &str,
    body: ContactCatalogOwnerRequest,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = require_ctx(&req)?;
    let party_id = crate::handlers::catalog::actor_party_id(&req)?;

    let cmd = ContactCatalogOwnerCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: party_id,
        is_admin: is_catalog_admin(&ctx),
        item_type: item_type.to_string(),
        item_id,
        message: body.message,
    };

    let result = state.contact_catalog_owner.execute(cmd).await?;
    Ok(HttpResponse::Created().json(result))
}
