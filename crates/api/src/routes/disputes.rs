use crate::handlers::disputes::{
    admin_escalate_dispute, admin_list_disputes, admin_reject_dispute, admin_resolve_dispute,
    create_dispute, get_dispute, list_deal_disputes, respond_to_dispute, submit_evidence,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/deals/{deal_id}/disputes")
            .route(web::post().to(create_dispute::create_dispute))
            .route(web::get().to(list_deal_disputes::list_deal_disputes)),
    )
    .service(web::resource("/disputes/{dispute_id}").route(web::get().to(get_dispute::get_dispute)))
    .service(
        web::resource("/disputes/{dispute_id}/evidence")
            .route(web::post().to(submit_evidence::submit_evidence)),
    )
    .service(
        web::resource("/disputes/{dispute_id}/responses")
            .route(web::post().to(respond_to_dispute::respond_to_dispute)),
    )
    .service(
        web::resource("/admin/disputes")
            .route(web::get().to(admin_list_disputes::admin_list_disputes)),
    )
    .service(
        web::resource("/admin/disputes/{dispute_id}/escalate")
            .route(web::post().to(admin_escalate_dispute::admin_escalate_dispute)),
    )
    .service(
        web::resource("/admin/disputes/{dispute_id}/resolve")
            .route(web::post().to(admin_resolve_dispute::admin_resolve_dispute)),
    )
    .service(
        web::resource("/admin/disputes/{dispute_id}/reject")
            .route(web::post().to(admin_reject_dispute::admin_reject_dispute)),
    );
}
