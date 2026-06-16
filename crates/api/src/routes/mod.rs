use actix_web::{web, HttpResponse};

pub mod chatrooms;
pub mod deals;
pub mod disputes;
pub mod messages;
pub mod notifications;
pub mod parties;
pub mod payments;
pub mod reviews;
pub mod users;
pub mod verifications;

pub mod admin;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/api/v1/ws/messages",
        web::get().to(crate::websocket::message_socket::ws_handler),
    );
    cfg.service(
        web::scope("/api/v1")
            .wrap(crate::middleware::auth::Authentication)
            .configure(users::configure)
            .configure(parties::configure)
            .configure(deals::configure)
            .configure(disputes::configure)
            .configure(messages::configure)
            .configure(notifications::configure)
            .configure(chatrooms::configure)
            .configure(payments::configure)
            .configure(reviews::configure)
            .configure(verifications::configure)
            .configure(admin::configure)
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
            .route(
                "/auth/forgot-password",
                web::post().to(crate::handlers::forgot_password::forgot_password),
            )
            .route(
                "/auth/reset-password",
                web::post().to(crate::handlers::reset_password::reset_password),
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
