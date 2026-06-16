use crate::errors::ApiError;
use crate::handlers::notifications::{extract_ctx, require_notification_admin};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use application::notifications::dto::NotificationTemplateRequest;
use uuid::Uuid;

pub async fn admin_list_templates(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    require_notification_admin(&ctx)?;

    let result = state.admin_list_templates.execute(None, None).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn admin_create_template(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<NotificationTemplateRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    require_notification_admin(&ctx)?;

    let result = state
        .admin_create_template
        .execute(body.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn admin_get_template(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    require_notification_admin(&ctx)?;

    let result = state.admin_get_template.execute(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn admin_update_template(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<NotificationTemplateRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    require_notification_admin(&ctx)?;

    let result = state
        .admin_update_template
        .execute(path.into_inner(), body.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn admin_delete_template(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = extract_ctx(&req)?;
    require_notification_admin(&ctx)?;

    state
        .admin_delete_template
        .execute(path.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
