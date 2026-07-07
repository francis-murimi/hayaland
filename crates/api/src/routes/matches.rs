use crate::handlers::matches::{
    admin, generate_matches, list_matches, respond_to_match, status_counts,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/matches")
            .route(web::get().to(list_matches::list_matches))
            .route(web::post().to(generate_matches::generate_matches)),
    )
    .service(
        web::resource("/matches/generate")
            .route(web::post().to(generate_matches::generate_matches)),
    )
    .service(
        web::resource("/matches/counts").route(web::get().to(status_counts::get_status_counts)),
    )
    .service(
        web::resource("/matches/{id}/respond")
            .route(web::post().to(respond_to_match::respond_to_match)),
    )
    .service(
        web::resource("/admin/matches")
            .route(web::get().to(admin::list_all_matches))
            .route(web::delete().to(admin::delete_all_match_suggestions)),
    )
    .service(
        web::resource("/admin/matches/counts")
            .route(web::get().to(admin::get_platform_status_counts)),
    )
    .service(
        web::resource("/admin/matches/{id}/status")
            .route(web::patch().to(admin::update_match_status)),
    )
    .service(
        web::resource("/admin/parties/{id}/matches")
            .route(web::delete().to(admin::delete_match_suggestions_for_party)),
    );
}
