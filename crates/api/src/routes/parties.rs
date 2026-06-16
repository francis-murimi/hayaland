use crate::handlers::parties::{
    add_role, create_party, delete_party, get_party, list_my_parties, list_parties, list_roles,
    nearby, remove_role, search_parties, update_party,
};
use crate::handlers::trust_scores::{get_trust_score, recalculate_trust_score};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/parties")
            .route(web::post().to(create_party::create_party))
            .route(web::get().to(list_parties::list_parties)),
    )
    .service(web::resource("/parties/me").route(web::get().to(list_my_parties::list_my_parties)))
    .service(web::resource("/parties/search").route(web::get().to(search_parties::search_parties)))
    .service(web::resource("/parties/nearby").route(web::get().to(nearby::nearby_parties)))
    .service(
        web::resource("/parties/{id}")
            .route(web::get().to(get_party::get_party))
            .route(web::put().to(update_party::update_party))
            .route(web::patch().to(update_party::update_party))
            .route(web::delete().to(delete_party::delete_party)),
    )
    .service(
        web::resource("/parties/{id}/roles")
            .route(web::get().to(list_roles::list_roles))
            .route(web::post().to(add_role::add_role)),
    )
    .service(
        web::resource("/parties/{id}/roles/{role_type}")
            .route(web::delete().to(remove_role::remove_role)),
    )
    .service(
        web::resource("/parties/{id}/trust").route(web::get().to(get_trust_score::get_trust_score)),
    )
    .service(
        web::resource("/parties/{id}/trust/recalculate")
            .route(web::post().to(recalculate_trust_score::recalculate_trust_score)),
    );
}
