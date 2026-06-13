use crate::handlers::deals::{
    create_deal, execute_transition, get_deal, list_deals, submit_deal, terms, update_deal,
    validate_deal, value_distribution,
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
    )
    .service(
        web::resource("/deals/{id}/terms")
            .route(web::post().to(terms::propose_term))
            .route(web::get().to(terms::list_terms)),
    )
    .service(
        web::resource("/deals/{id}/terms/{term_id}/counter")
            .route(web::post().to(terms::counter_term)),
    )
    .service(
        web::resource("/deals/{id}/terms/{term_id}/accept")
            .route(web::post().to(terms::accept_term)),
    )
    .service(
        web::resource("/deals/{id}/terms/{term_id}/reject")
            .route(web::post().to(terms::reject_term)),
    )
    .service(
        web::resource("/deals/{id}/terms/{term_id}/withdraw")
            .route(web::post().to(terms::withdraw_term)),
    )
    .service(
        web::resource("/deals/{id}/value-distribution")
            .route(web::post().to(value_distribution::set_value_distribution))
            .route(web::get().to(value_distribution::get_value_distribution)),
    )
    .service(
        web::resource("/deals/{id}/validate").route(web::post().to(validate_deal::validate_deal)),
    );
}
