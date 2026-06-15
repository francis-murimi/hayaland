use crate::dto::ListConversationsQueryParams;
use crate::errors::ApiError;
use crate::handlers::messages::{actor_party_id, extract_ctx};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::messages::dto::ListConversationsQuery;

pub async fn list_conversations(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ListConversationsQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    let actor_party_id = actor_party_id(&req);

    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let page = query.page.unwrap_or(1).max(1);

    let app_query = ListConversationsQuery {
        actor_user_id: ctx.user_id,
        actor_party_id,
        limit: per_page,
        offset: (page - 1) * per_page,
    };

    let results = state.list_conversations.execute(app_query).await?;
    Ok(HttpResponse::Ok().json(results))
}
