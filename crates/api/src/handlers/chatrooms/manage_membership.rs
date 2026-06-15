use crate::dto::SetRoleRequest;
use crate::errors::ApiError;
use crate::handlers::chatrooms::{extract_ctx, is_chatroom_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::chatrooms::dto::{ManageMembershipCommand, MembershipAction};
use uuid::Uuid;
use validator::Validate;

pub async fn remove_member(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    let (room_id, membership_id) = path.into_inner();

    let cmd = ManageMembershipCommand {
        actor_user_id: ctx.user_id,
        scopes: ctx.scopes.clone(),
        is_admin: is_chatroom_admin(&ctx),
        room_id,
        action: MembershipAction::Remove,
        target_user_id: None,
        target_party_id: None,
        role: None,
        membership_id: Some(membership_id),
    };

    state.manage_chat_room_membership.execute(cmd).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn set_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<SetRoleRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = extract_ctx(&req)?;
    let (room_id, membership_id) = path.into_inner();

    let cmd = ManageMembershipCommand {
        actor_user_id: ctx.user_id,
        scopes: ctx.scopes.clone(),
        is_admin: is_chatroom_admin(&ctx),
        room_id,
        action: MembershipAction::SetRole,
        target_user_id: None,
        target_party_id: None,
        role: Some(body.role),
        membership_id: Some(membership_id),
    };

    state.manage_chat_room_membership.execute(cmd).await?;
    Ok(HttpResponse::NoContent().finish())
}
