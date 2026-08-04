use crate::handlers::analytics::{
    get_dashboard_summary, get_deal_trends, get_party_activity, list_daily_metrics,
    refresh_daily_metrics,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/analytics")
            .route("/dashboard", web::get().to(get_dashboard_summary))
            .route("/trends", web::get().to(get_deal_trends))
            .route("/activity", web::get().to(get_party_activity))
            .route("/metrics", web::get().to(list_daily_metrics))
            .route("/refresh", web::post().to(refresh_daily_metrics)),
    );
}
