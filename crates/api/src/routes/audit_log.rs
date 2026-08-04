use crate::handlers::audit_log::{list_audit_log, record_admin_action};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/audit-log")
            .route("", web::get().to(list_audit_log))
            .route("", web::post().to(record_admin_action)),
    );
}
