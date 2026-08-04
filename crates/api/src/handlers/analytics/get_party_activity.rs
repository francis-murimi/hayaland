use crate::errors::ApiError;
use crate::middleware::auth::require_any_scope;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::analytics::dto::DateRangeQuery;
use application::users::token::AuthContext;

pub async fn get_party_activity(
    state: web::Data<AppState>,
    query: web::Query<DateRangeQuery>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    require_any_scope(&ctx, &["admin:analytics", "admin:*"])?;
    let q = query.into_inner();
    let activity = state.get_party_activity.execute(q.from, q.to).await?;
    Ok(HttpResponse::Ok().json(activity))
}
