use crate::handlers::agreements::{admin_get_agreement, admin_update_agreement};
use crate::handlers::parties::update_party::admin_update_party;
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/admin/deals/{id}/agreement")
            .route(web::get().to(admin_get_agreement::admin_get_agreement))
            .route(web::patch().to(admin_update_agreement::admin_update_agreement)),
    )
    .service(
        web::resource("/admin/parties/{id}")
            .route(web::patch().to(admin_update_party))
            .route(web::put().to(admin_update_party)),
    );
}
