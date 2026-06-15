use crate::handlers::messages::{
    admin_broadcast, delete_message, edit_message, get_message, list_conversations, list_messages,
    mark_read, pin_message, react, remove_reaction, send_message, unpin_message, unread_count,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/messages").route(web::post().to(send_message::send_message)))
        .service(
            web::resource("/conversations")
                .route(web::get().to(list_conversations::list_conversations)),
        )
        .service(
            web::resource("/conversations/{id}/messages")
                .route(web::get().to(list_messages::list_messages)),
        )
        .service(
            web::resource("/messages/unread-count")
                .route(web::get().to(unread_count::unread_count)),
        )
        .service(
            web::resource("/messages/{id}")
                .route(web::get().to(get_message::get_message))
                .route(web::patch().to(edit_message::edit_message))
                .route(web::delete().to(delete_message::delete_message)),
        )
        .service(web::resource("/messages/{id}/read").route(web::post().to(mark_read::mark_read)))
        .service(web::resource("/messages/{id}/reactions").route(web::post().to(react::react)))
        .service(
            web::resource("/messages/{id}/reactions/{reaction_type}")
                .route(web::delete().to(remove_reaction::remove_reaction)),
        )
        .service(
            web::resource("/messages/{id}/pin").route(web::post().to(pin_message::pin_message)),
        )
        .service(
            web::resource("/messages/{id}/unpin")
                .route(web::post().to(unpin_message::unpin_message)),
        )
        .service(
            web::resource("/admin/messages/broadcast")
                .route(web::post().to(admin_broadcast::admin_broadcast)),
        );
}
