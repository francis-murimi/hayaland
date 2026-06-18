use crate::handlers::catalog::{
    admin, categories, contact, discovery, enhancements, needs, resources, settings,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    // Resources
    cfg.service(
        web::resource("/resources")
            .route(web::post().to(resources::create_resource))
            .route(web::get().to(resources::list_resources)),
    )
    .service(web::resource("/resources/search").route(web::get().to(resources::search_resources)))
    .service(
        web::resource("/resources/categories")
            .route(web::get().to(categories::list_resource_categories)),
    )
    .service(
        web::resource("/resources/{id}")
            .route(web::get().to(resources::get_resource))
            .route(web::put().to(resources::update_resource))
            .route(web::patch().to(resources::update_resource))
            .route(web::delete().to(resources::delete_resource)),
    )
    // Needs
    .service(
        web::resource("/needs")
            .route(web::post().to(needs::create_need))
            .route(web::get().to(needs::list_needs)),
    )
    .service(web::resource("/needs/search").route(web::get().to(needs::search_needs)))
    .service(
        web::resource("/needs/categories").route(web::get().to(categories::list_need_categories)),
    )
    .service(
        web::resource("/needs/{id}")
            .route(web::get().to(needs::get_need))
            .route(web::put().to(needs::update_need))
            .route(web::patch().to(needs::update_need))
            .route(web::delete().to(needs::delete_need)),
    )
    // Enhancements
    .service(
        web::resource("/enhancements")
            .route(web::post().to(enhancements::create_enhancement))
            .route(web::get().to(enhancements::list_enhancements)),
    )
    .service(
        web::resource("/enhancements/search")
            .route(web::get().to(enhancements::search_enhancements)),
    )
    .service(
        web::resource("/enhancements/categories")
            .route(web::get().to(categories::list_enhancement_categories)),
    )
    .service(
        web::resource("/enhancements/{id}")
            .route(web::get().to(enhancements::get_enhancement))
            .route(web::put().to(enhancements::update_enhancement))
            .route(web::patch().to(enhancements::update_enhancement))
            .route(web::delete().to(enhancements::delete_enhancement)),
    )
    // All category tree
    .service(
        web::resource("/catalog/categories").route(web::get().to(categories::list_all_categories)),
    )
    // Discovery
    .service(web::resource("/discovery/domains").route(web::get().to(discovery::list_domains)))
    .service(web::resource("/discovery/domains/{id}").route(web::get().to(discovery::get_domain)))
    // Admin flags
    .service(
        web::resource("/admin/catalog/resources/{id}/flags")
            .route(web::patch().to(admin::update_resource_flags)),
    )
    .service(
        web::resource("/admin/catalog/needs/{id}/flags")
            .route(web::patch().to(admin::update_need_flags)),
    )
    .service(
        web::resource("/admin/catalog/enhancements/{id}/flags")
            .route(web::patch().to(admin::update_enhancement_flags)),
    )
    // Contact owner
    .service(
        web::resource("/catalog/resources/{id}/contact")
            .route(web::post().to(contact::contact_resource_owner)),
    )
    .service(
        web::resource("/catalog/needs/{id}/contact")
            .route(web::post().to(contact::contact_need_owner)),
    )
    .service(
        web::resource("/catalog/enhancements/{id}/contact")
            .route(web::post().to(contact::contact_enhancement_owner)),
    )
    // Party catalog settings
    .service(
        web::resource("/parties/{id}/catalog-settings")
            .route(web::patch().to(settings::update_party_catalog_settings)),
    );
}
