use crate::errors::ApiError;
use crate::handlers::media::{delete_media, download_media, list_media, upload_media};
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::users::token::AuthContext;
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
    _state: web::Data<AppState>,
    _path: web::Path<Uuid>,
    _ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    Err(ApiError::Validation(
        "use /uploads/{storage_path} to download files".into(),
    ))
}
