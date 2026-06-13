use crate::dto::{AddPartyRoleRequest, PartyRolesResponse};
use crate::errors::ApiError;
use crate::handlers::parties::{is_admin, parse_role};
use crate::AppState;
use actix_web::HttpMessage;
use actix_web::{web, HttpRequest, HttpResponse};
use application::parties::dto::AddPartyRoleCommand;
use application::users::token::AuthContext;
use domain::entities::RoleProfile;
use uuid::Uuid;
use validator::Validate;

pub async fn add_role(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<AddPartyRoleRequest>,
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

    let role = parse_role(&body.role_type)?;
    let profile: RoleProfile = serde_json::from_value(body.profile.clone())
        .map_err(|e| ApiError::Validation(format!("invalid role profile: {e}")))?;

    // Ensure profile matches role type.
    if profile.role_type() != role {
        return Err(ApiError::Validation(
            "role profile type does not match role_type".to_string(),
        ));
    }

    let cmd = AddPartyRoleCommand {
        actor_user_id: ctx.user_id,
        is_admin: is_admin(&ctx),
        role,
        profile,
    };

    let result = state.add_party_role.execute(path.into_inner(), cmd).await?;
    Ok(HttpResponse::Created().json(PartyRolesResponse {
        roles: vec![result],
    }))
}
