use crate::handlers::chatrooms::{
    create_chat_room, delete_chat_room, get_chat_room, join_chat_room, leave_chat_room,
    list_chat_rooms, list_room_messages, manage_membership, update_chat_room,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/chatrooms")
            .route(web::get().to(list_chat_rooms::list_chat_rooms))
            .route(web::post().to(create_chat_room::create_chat_room)),
    )
    .service(
        web::resource("/chatrooms/{id}")
            .route(web::get().to(get_chat_room::get_chat_room))
            .route(web::patch().to(update_chat_room::update_chat_room))
            .route(web::delete().to(delete_chat_room::delete_chat_room)),
    )
    .service(
        web::resource("/chatrooms/{id}/members")
            .route(web::post().to(join_chat_room::join_chat_room)),
    )
    .service(
        web::resource("/chatrooms/{id}/members/me")
            .route(web::delete().to(leave_chat_room::leave_chat_room)),
    )
    .service(
        web::resource("/chatrooms/{id}/members/{membership_id}")
            .route(web::delete().to(manage_membership::remove_member)),
    )
    .service(
        web::resource("/chatrooms/{id}/members/{membership_id}/role")
            .route(web::patch().to(manage_membership::set_role)),
    )
    .service(
        web::resource("/chatrooms/{id}/messages")
            .route(web::get().to(list_room_messages::list_room_messages)),
    );
}
