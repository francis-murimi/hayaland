use crate::handlers::deals::{
    create_deal, execute_transition, get_deal, list_deals, submit_deal, update_deal,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/deals")
            .route(web::post().to(create_deal::create_deal))
            .route(web::get().to(list_deals::list_deals)),
    )
    .service(
        web::resource("/deals/{id}")
            .route(web::get().to(get_deal::get_deal))
            .route(web::put().to(update_deal::update_deal))
            .route(web::patch().to(update_deal::update_deal)),
    )
    .service(web::resource("/deals/{id}/submit").route(web::post().to(submit_deal::submit_deal)))
    .service(
        web::resource("/deals/{id}/transitions")
            .route(web::post().to(execute_transition::execute_transition)),
    );
}
