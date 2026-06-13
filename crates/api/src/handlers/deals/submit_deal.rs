use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::deals::dto::SubmitDealCommand;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::AppState;

pub async fn submit_deal(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;

    let cmd = SubmitDealCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
    };

    let result = state.submit_deal.execute(path.into_inner(), cmd).await?;
    Ok(HttpResponse::Ok().json(result))
}
