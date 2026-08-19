use crate::errors::ApiError;
use crate::handlers::media::{delete_media, download_media, list_media, upload_media};
use crate::middleware::auth::require_owner_or_admin;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::errors::ApplicationError;
use application::users::token::AuthContext;
use std::path::Component;
use uuid::Uuid;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/media")
            .route("", web::get().to(list_media))
            .route("", web::post().to(upload_media))
            .route("/{id}", web::delete().to(delete_media))
            .route("/{id}/download", web::get().to(download_media_by_id)),
    );
}

pub fn configure_uploads(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/uploads/{path:.*}").route(web::get().to(download_media)));
}

async fn download_media_by_id(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let upload = state
        .media_repo
        .find_by_id(id, false)
        .await
        .map_err(|e| ApiError::Application(e.into()))?
        .ok_or(ApiError::Application(ApplicationError::MediaNotFound))?;

    if !upload.is_public {
        require_owner_or_admin(&ctx, upload.owner_user_id)?;
    }

    let base = std::path::Path::new(&state.media_settings.storage_path);
    let mut cleaned = std::path::PathBuf::new();
    for component in std::path::Path::new(&upload.storage_path).components() {
        match component {
            Component::Normal(c) => cleaned.push(c),
            Component::RootDir => {}
            _ => return Err(ApiError::Forbidden),
        }
    }
    let final_path = base.join(&cleaned);

    let content = tokio::fs::read(&final_path)
        .await
        .map_err(|_| ApiError::Application(ApplicationError::MediaNotFound))?;

    Ok(HttpResponse::Ok()
        .content_type(upload.content_type)
        .body(content))
}
