use crate::dto::{PartiesResponse, SearchPartiesQuery};
use crate::errors::ApiError;
use crate::handlers::parties::parse_role;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;
use actix_web::HttpMessage;
use actix_web::{web, HttpResponse};
use application::parties::dto::SearchPartiesQuery as AppQuery;
use application::users::token::AuthContext;
use domain::entities::{PartyType, VerificationStatus};
use validator::Validate;

pub async fn search_parties(
    state: web::Data<AppState>,
    query: web::Query<SearchPartiesQuery>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, ApiError> {
    query.validate().map_err(ApiError::from)?;

    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "parties:read", "admin:parties")?;

    let roles = match &query.roles {
        Some(roles) => roles
            .iter()
            .map(|r| parse_role(r))
            .collect::<Result<Vec<_>, _>>()?,
        None => vec![],
    };

    let party_types = match &query.party_types {
        Some(types) => types
            .iter()
            .map(|t| PartyType::try_from(t.as_str()).map_err(|e| ApiError::Application(e.into())))
            .collect::<Result<Vec<_>, _>>()?,
        None => vec![],
    };

    let verification_statuses = match &query.verification_statuses {
        Some(statuses) => statuses
            .iter()
            .map(|s| {
                VerificationStatus::try_from(s.as_str())
                    .map_err(|e| ApiError::Application(e.into()))
            })
            .collect::<Result<Vec<_>, _>>()?,
        None => vec![],
    };

    if query.radius_km.is_some() && (query.latitude.is_none() || query.longitude.is_none()) {
        return Err(ApiError::Validation(
            "radiusKm requires both lat and lng".to_string(),
        ));
    }

    let app_query = AppQuery {
        query: query.q.clone(),
        roles,
        party_types,
        verification_statuses,
        min_trust_score: query.min_trust_score,
        max_trust_score: query.max_trust_score,
        primary_domain_id: query.primary_domain_id,
        active_only: query.active_only,
        latitude: query.latitude,
        longitude: query.longitude,
        radius_km: query.radius_km,
        limit: query.limit.unwrap_or(20),
        offset: query.offset.unwrap_or(0),
    };

    let result = state.search_parties.execute(app_query).await?;
    Ok(HttpResponse::Ok().json(PartiesResponse::from(result)))
}
