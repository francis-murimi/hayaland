use crate::handlers::verifications::{
    admin_approve_verification, admin_list_verifications, admin_reject_verification,
    admin_revoke_verification, create_verification, get_verification_status,
    list_party_verifications,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/parties/{party_id}/verifications")
            .route(web::post().to(create_verification::create_verification))
            .route(web::get().to(list_party_verifications::list_party_verifications)),
    )
    .service(
        web::resource("/parties/{party_id}/verifications/status")
            .route(web::get().to(get_verification_status::get_verification_status)),
    )
    .service(
        web::resource("/admin/verifications")
            .route(web::get().to(admin_list_verifications::admin_list_verifications)),
    )
    .service(
        web::resource("/admin/verifications/{verification_id}/approve")
            .route(web::post().to(admin_approve_verification::admin_approve_verification)),
    )
    .service(
        web::resource("/admin/verifications/{verification_id}/reject")
            .route(web::post().to(admin_reject_verification::admin_reject_verification)),
    )
    .service(
        web::resource("/admin/verifications/{verification_id}/revoke")
            .route(web::post().to(admin_revoke_verification::admin_revoke_verification)),
    );
}
