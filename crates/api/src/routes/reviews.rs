use crate::handlers::reviews::{
    admin_hide_review, admin_list_reviews, create_review, get_deal_review_status, get_review,
    list_deal_reviews, list_party_reviews,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/deals/{deal_id}/reviews")
            .route(web::post().to(create_review::create_review))
            .route(web::get().to(list_deal_reviews::list_deal_reviews)),
    )
    .service(
        web::resource("/deals/{deal_id}/reviews/status")
            .route(web::get().to(get_deal_review_status::get_deal_review_status)),
    )
    .service(
        web::resource("/parties/{party_id}/reviews")
            .route(web::get().to(list_party_reviews::list_party_reviews)),
    )
    .service(web::resource("/reviews/{review_id}").route(web::get().to(get_review::get_review)))
    .service(
        web::resource("/admin/reviews")
            .route(web::get().to(admin_list_reviews::admin_list_reviews)),
    )
    .service(
        web::resource("/admin/reviews/{review_id}/hide")
            .route(web::post().to(admin_hide_review::admin_hide_review)),
    );
}
