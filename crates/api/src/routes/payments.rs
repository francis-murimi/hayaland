use crate::handlers::payments::{
    deposit_points, get_deal_wallet, get_wallet, list_deal_transactions, list_wallet_transactions,
    withdraw_points,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/parties/{id}/wallet").route(web::get().to(get_wallet::get_wallet)))
        .service(
            web::resource("/parties/{id}/wallet/deposits")
                .route(web::post().to(deposit_points::deposit_points)),
        )
        .service(
            web::resource("/parties/{id}/wallet/withdrawals")
                .route(web::post().to(withdraw_points::withdraw_points)),
        )
        .service(
            web::resource("/parties/{id}/wallet/transactions")
                .route(web::get().to(list_wallet_transactions::list_wallet_transactions)),
        )
        .service(
            web::resource("/parties/{party_id}/deals/{deal_id}/wallet")
                .route(web::get().to(get_deal_wallet::get_deal_wallet)),
        )
        .service(
            web::resource("/parties/{party_id}/deals/{deal_id}/transactions")
                .route(web::get().to(list_deal_transactions::list_deal_transactions)),
        );
}
