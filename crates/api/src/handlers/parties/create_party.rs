use crate::dto::{CreatePartyRequest, PartyResponse};
use crate::errors::ApiError;
use crate::handlers::parties::parse_role;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;
use actix_web::HttpMessage;
use actix_web::{http::header::LOCATION, web, HttpRequest, HttpResponse};
use application::parties::dto::CreatePartyCommand;
use application::users::token::AuthContext;
use domain::entities::PartyType;
use validator::Validate;

pub async fn create_party(
    state: web::Data<AppState>,
    body: web::Json<CreatePartyRequest>,
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

    let party_type = PartyType::try_from(body.party_type.as_str())
        .map_err(|e| ApiError::Application(e.into()))?;

    let roles = body
        .roles
        .iter()
        .map(|r| parse_role(r))
        .collect::<Result<Vec<_>, _>>()?;

    let cmd = CreatePartyCommand {
        actor_user_id: ctx.user_id,
        party_type,
        display_name: body.display_name.clone(),
        email: body.email.clone(),
        phone: body.phone.clone(),
        tax_id: body.tax_id.clone(),
        primary_domain_id: body.primary_domain_id,
        latitude: body.latitude,
        longitude: body.longitude,
        service_radius_km: body.service_radius_km,
        roles,
    };

    let result = state.create_party.execute(cmd).await?;
    let location = format!("/api/v1/parties/{}", result.id);

    Ok(HttpResponse::Created()
        .insert_header((LOCATION, location))
        .json(PartyResponse::from(result)))
}
