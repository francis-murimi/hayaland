use crate::handlers::notifications;
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/notifications")
            .route(web::get().to(notifications::list_notifications::list_notifications)),
    )
    .service(
        web::resource("/notifications/unread-count")
            .route(web::get().to(notifications::unread_count::unread_count)),
    )
    .service(
        web::resource("/notifications/actions/mark-all-read")
            .route(web::post().to(notifications::mark_all_read::mark_all_read)),
    )
    .service(
        web::resource("/notifications/preferences")
            .route(web::get().to(notifications::preferences::get_preferences))
            .route(web::put().to(notifications::preferences::update_preferences)),
    )
    .service(
        web::resource("/notifications/{id}")
            .route(web::get().to(notifications::get_notification::get_notification))
            .route(web::patch().to(notifications::mark_read::mark_read))
            .route(web::delete().to(notifications::delete_notification::delete_notification)),
    )
    .service(
        web::resource("/admin/notifications/send")
            .route(web::post().to(notifications::admin_send::admin_send)),
    )
    .service(
        web::resource("/admin/notification-templates")
            .route(web::get().to(notifications::admin_templates::admin_list_templates))
            .route(web::post().to(notifications::admin_templates::admin_create_template)),
    )
    .service(
        web::resource("/admin/notification-templates/{id}")
            .route(web::get().to(notifications::admin_templates::admin_get_template))
            .route(web::put().to(notifications::admin_templates::admin_update_template))
            .route(web::delete().to(notifications::admin_templates::admin_delete_template)),
    );
}
