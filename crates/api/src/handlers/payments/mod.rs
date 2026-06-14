pub mod approve_transaction;
pub mod deposit_points;
pub mod dto;
pub mod get_deal_wallet;
pub mod get_transaction;
pub mod get_wallet;
pub mod list_deal_transactions;
pub mod list_pending_approvals;
pub mod list_wallet_transactions;
pub mod reject_transaction;
pub mod withdraw_points;

use application::users::token::AuthContext;

pub(crate) fn is_transaction_admin(ctx: &AuthContext) -> bool {
    ctx.has_role("admin") || ctx.has_scope("admin:transactions") || ctx.has_scope("admin:*")
}
