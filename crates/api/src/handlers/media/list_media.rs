use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::media::dto::ListMediaCommand;
use application::users::token::AuthContext;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize, Default)]
pub struct ListMediaQuery {
    pub owner_user_id: Option<Uuid>,
    pub owner_party_id: Option<Uuid>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<Uuid>,
    pub include_deleted: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_media(
    state: web::Data<AppState>,
    query: web::Query<ListMediaQuery>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    let is_admin = ctx.has_scope("admin:media") || ctx.has_scope("admin:*");
    let owner_user_id = if is_admin {
        query.owner_user_id
    } else {
        Some(ctx.user_id)
    };

    let cmd = ListMediaCommand {
        owner_user_id,
        owner_party_id: query.owner_party_id,
        related_entity_type: query.related_entity_type.clone(),
        related_entity_id: query.related_entity_id,
        include_deleted: query.include_deleted,
        limit: query.limit,
        offset: query.offset,
    };

    let result = state
        .list_media
        .execute(cmd, state.media_storage.clone())
        .await?;
    Ok(HttpResponse::Ok().json(result))
}
