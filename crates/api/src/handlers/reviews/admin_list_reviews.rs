use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::reviews::dto::AdminReviewListQuery as AppQuery;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::reviews::dto::ReviewsResponse;
use crate::middleware::auth::require_any_scope;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct Query {
    #[serde(rename = "dealId")]
    pub deal_id: Option<Uuid>,
    #[serde(rename = "reviewerPartyId")]
    pub reviewer_party_id: Option<Uuid>,
    #[serde(rename = "reviewedPartyId")]
    pub reviewed_party_id: Option<Uuid>,
    #[serde(rename = "isPublic")]
    pub is_public: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn admin_list_reviews(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<Query>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_any_scope(&ctx, &["admin:reviews", "admin:*"])?;

    let app_query = AppQuery {
        deal_id: query.deal_id,
        reviewer_party_id: query.reviewer_party_id,
        reviewed_party_id: query.reviewed_party_id,
        is_public: query.is_public,
        limit: query.limit.unwrap_or(20).clamp(1, 100),
        offset: query.offset.unwrap_or(0).max(0),
    };

    let result = state.list_admin_reviews.execute(app_query).await?;
    Ok(HttpResponse::Ok().json(ReviewsResponse::from(result)))
}
