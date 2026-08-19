use crate::handlers::{
    assign_user_roles::assign_user_roles, create_user::create_user,
    deactivate_user::deactivate_user, get_user::get_user, list_users::list_users,
    register_push_token::register_push_token, update_user::update_user,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/users")
            .route(web::post().to(create_user))
            .route(web::get().to(list_users)),
    )
    .service(
        web::resource("/users/{id}")
            .route(web::get().to(get_user))
            .route(web::patch().to(update_user))
            .route(web::delete().to(deactivate_user)),
    )
    .service(web::resource("/users/{id}/roles").route(web::patch().to(assign_user_roles)))
    .service(web::resource("/users/me/push-tokens").route(web::post().to(register_push_token)));
}
