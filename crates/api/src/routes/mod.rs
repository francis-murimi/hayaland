use actix_web::{web, HttpResponse};

pub mod users;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .wrap(crate::middleware::auth::Authentication)
            .configure(users::configure)
            .route("/health", web::get().to(health))
            .route("/auth/login", web::post().to(crate::handlers::login::login)),
    );
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}
