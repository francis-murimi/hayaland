use crate::dto::catalog::UpdatePartyCatalogSettingsRequest;
use crate::errors::ApiError;
use crate::handlers::catalog::{is_catalog_admin, require_ctx};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::catalog::dto::UpdatePartyCatalogSettingsCommand;
use uuid::Uuid;
use validator::Validate;

pub async fn update_party_catalog_settings(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<UpdatePartyCatalogSettingsRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = require_ctx(&req)?;
    let party_id = crate::handlers::catalog::actor_party_id(&req)?;

    let cmd = UpdatePartyCatalogSettingsCommand {
        actor_user_id: ctx.user_id,
        actor_party_id: party_id,
        is_admin: is_catalog_admin(&ctx),
        accepts_catalog_inquiries: body.accepts_catalog_inquiries,
        public_contact_email: body.public_contact_email,
    };

    let result = state
        .update_party_catalog_settings
        .execute(path.into_inner(), cmd)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
