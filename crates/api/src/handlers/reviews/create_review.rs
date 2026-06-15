use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::reviews::dto::SubmitReviewCommand;
use application::users::token::AuthContext;
use uuid::Uuid;
use validator::Validate;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::reviews::dto::{CreateReviewRequest, ReviewResponse};
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn create_review(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<CreateReviewRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "reviews:write", "admin:reviews")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let is_admin = ctx.has_scope("admin:reviews") || ctx.has_scope("admin:*");

    let cmd = SubmitReviewCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        is_admin,
        deal_id: path.into_inner(),
        reviewed_party_id: body.reviewed_party_id,
        overall_rating: body.overall_rating,
        communication_rating: body.communication_rating,
        reliability_rating: body.reliability_rating,
        quality_rating: body.quality_rating,
        timeliness_rating: body.timeliness_rating,
        review_text: body.review_text.clone(),
        is_public: body.is_public,
    };

    let result = state.submit_review.execute(cmd).await?;
    Ok(HttpResponse::Created().json(ReviewResponse::from(result)))
}
