use crate::dto::{PartyResponse, UpdatePartyRequest};
use crate::errors::ApiError;
use crate::handlers::parties::is_admin;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;
use actix_web::HttpMessage;
use actix_web::{web, HttpRequest, HttpResponse};
use application::parties::dto::UpdatePartyCommand;
use application::users::token::AuthContext;
use domain::entities::VerificationStatus;
use uuid::Uuid;
use validator::Validate;

pub async fn update_party(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<UpdatePartyRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "parties:write", "admin:parties")?;

    let admin = is_admin(&ctx);

    let cmd = UpdatePartyCommand {
        actor_user_id: ctx.user_id,
        is_admin: admin,
        display_name: body.display_name.clone(),
        email: body.email.clone(),
        phone: body.phone.clone(),
        tax_id: body.tax_id.clone(),
        primary_domain_id: body.primary_domain_id,
        latitude: body.latitude,
        longitude: body.longitude,
        service_radius_km: body.service_radius_km,
        verification_status: None,
        is_active: None,
    };

    let result = state.update_party.execute(path.into_inner(), cmd).await?;
    Ok(HttpResponse::Ok().json(PartyResponse::from(result)))
}

pub async fn admin_update_party(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<serde_json::Value>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    if !is_admin(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let display_name = body
        .get("display_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let email = body
        .get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let phone = body
        .get("phone")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let tax_id = body
        .get("tax_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let primary_domain_id = body
        .get("primary_domain_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());
    let latitude = body.get("latitude").and_then(|v| v.as_f64());
    let longitude = body.get("longitude").and_then(|v| v.as_f64());
    let service_radius_km = body.get("service_radius_km").and_then(|v| v.as_f64());
    let verification_status = body
        .get("verification_status")
        .and_then(|v| v.as_str())
        .and_then(|s| VerificationStatus::try_from(s).ok());
    let is_active = body.get("is_active").and_then(|v| v.as_bool());

    let cmd = UpdatePartyCommand {
        actor_user_id: ctx.user_id,
        is_admin: true,
        display_name,
        email,
        phone,
        tax_id,
        primary_domain_id,
        latitude,
        longitude,
        service_radius_km,
        verification_status,
        is_active,
    };

    let result = state.update_party.execute(path.into_inner(), cmd).await?;
    Ok(HttpResponse::Ok().json(PartyResponse::from(result)))
}
