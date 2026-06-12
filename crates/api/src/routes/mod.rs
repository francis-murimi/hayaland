use actix_web::{web, HttpResponse};

pub mod users;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .wrap(crate::middleware::auth::Authentication)
            .configure(users::configure)
            .route("/health", web::get().to(health))
            .route("/auth/login", web::post().to(crate::handlers::login::login))
            .route(
                "/auth/verify-email",
                web::get().to(crate::handlers::verify_email::verify_email),
            )
            .route(
                "/auth/resend-verification",
                web::post().to(crate::handlers::resend_verification::resend_verification),
            )
            .route("/roles", web::get().to(crate::handlers::roles::list_roles))
            .route(
                "/roles/{name}",
                web::put().to(crate::handlers::roles::update_role_scopes),
            ),
    );
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}
