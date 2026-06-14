use crate::handlers::payments::{
    approve_transaction, deposit_points, get_deal_wallet, get_transaction, get_wallet,
    list_deal_transactions, list_pending_approvals, list_wallet_transactions, reject_transaction,
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
        )
        .service(
            web::resource("/payments/transactions/pending-approvals")
                .route(web::get().to(list_pending_approvals::list_pending_approvals)),
        )
        .service(
            web::resource("/payments/transactions/{id}")
                .route(web::get().to(get_transaction::get_transaction)),
        )
        .service(
            web::resource("/payments/transactions/{id}/approve")
                .route(web::post().to(approve_transaction::approve_transaction)),
        )
        .service(
            web::resource("/payments/transactions/{id}/reject")
                .route(web::post().to(reject_transaction::reject_transaction)),
        );
}
