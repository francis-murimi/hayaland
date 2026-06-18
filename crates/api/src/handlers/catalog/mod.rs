use crate::errors::ApiError;
use actix_web::{HttpMessage, HttpRequest};
use application::errors::ApplicationError;
use application::users::token::AuthContext;
use uuid::Uuid;

pub mod admin;
pub mod categories;
pub mod contact;
pub mod discovery;
pub mod enhancements;
pub mod needs;
pub mod resources;
pub mod settings;

pub(crate) fn extract_optional_ctx(req: &HttpRequest) -> Option<AuthContext> {
    req.extensions().get::<AuthContext>().cloned()
}

pub(crate) fn require_ctx(req: &HttpRequest) -> Result<AuthContext, ApiError> {
    extract_optional_ctx(req).ok_or(ApiError::Application(ApplicationError::Unauthorized))
}

pub(crate) fn is_catalog_admin(ctx: &AuthContext) -> bool {
    ctx.has_scope("admin:catalog") || ctx.has_scope("admin:*")
}

pub(crate) fn actor_party_id(req: &HttpRequest) -> Result<Uuid, ApiError> {
    let value = req
        .headers()
        .get("X-Party-ID")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::Validation("X-Party-ID header is required".to_string()))?;
    value
        .parse::<Uuid>()
        .map_err(|_| ApiError::Validation("invalid X-Party-ID header".to_string()))
}

/// For public read endpoints the party header is optional; for mutations it is required.
pub(crate) fn optional_actor_party_id(req: &HttpRequest) -> Option<Uuid> {
    req.headers()
        .get("X-Party-ID")
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.parse::<Uuid>().ok())
}

/// Returns the acting party ID and admin flag for a request that may be anonymous.
pub(crate) fn catalog_actor(req: &HttpRequest) -> Result<(Option<Uuid>, bool), ApiError> {
    match extract_optional_ctx(req) {
        Some(ctx) => {
            let admin = is_catalog_admin(&ctx);
            let party_id = optional_actor_party_id(req);
            Ok((party_id, admin))
        }
        None => Ok((None, false)),
    }
}
