use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::errors::ApplicationError;
use std::path::Component;

pub async fn download_media(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let relative = path.into_inner();
    if relative.is_empty() || relative.starts_with('/') {
        return Err(ApiError::Application(ApplicationError::MediaNotFound));
    }

    let base = std::path::Path::new(&state.media_settings.storage_path);
    let mut cleaned = std::path::PathBuf::new();
    for component in std::path::Path::new(&relative).components() {
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
        .content_type(mime::APPLICATION_OCTET_STREAM)
        .body(content))
}
