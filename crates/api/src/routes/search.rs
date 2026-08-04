use crate::handlers::search::search;
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/search").route(web::get().to(search)));
}
