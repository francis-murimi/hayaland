use crate::dto::NotificationListQueryRequest;
use crate::errors::ApiError;
use crate::handlers::notifications::{actor_party_id, extract_ctx};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::notifications::dto::NotificationListQuery;

pub async fn list_notifications(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<NotificationListQueryRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    crate::middleware::auth::require_scope(&ctx, "notifications:read")?;

    let party_id = actor_party_id(&req);
    let query = NotificationListQuery {
        notification_type: query.notification_type,
        is_read: query.is_read,
        is_actioned: query.is_actioned,
        priority: query.priority,
        limit: query.limit,
        offset: query.offset,
    };

    let result = state
        .list_notifications
        .execute(ctx.user_id, party_id, query)
        .await?;

    Ok(HttpResponse::Ok().json(result))
}
