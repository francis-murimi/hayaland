use crate::errors::ApplicationError;
use crate::payments::dto::{
    ListPendingApprovalsQuery, ListPendingApprovalsResult, TransactionResult,
};
use domain::repositories::{PartyRepository, WalletRepository};
use std::sync::Arc;
use tracing::{info, instrument};

/// List transactions that are pending approval and awaiting a decision from
/// the acting party.
#[derive(Clone)]
pub struct ListPendingApprovals {
    party_repo: Arc<dyn PartyRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl ListPendingApprovals {
    pub fn new(
        party_repo: Arc<dyn PartyRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
    ) -> Self {
        Self {
            party_repo,
            wallet_repo,
        }
    }

    #[instrument(skip(self, query), fields(party_id = %query.actor_party_id))]
    pub async fn execute(
        &self,
        query: ListPendingApprovalsQuery,
    ) -> Result<ListPendingApprovalsResult, ApplicationError> {
        if !query.is_admin
            && !self
                .party_repo
                .is_user_member_of_party(query.actor_user_id, query.actor_party_id)
                .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        let offset = query.offset.unwrap_or(0).max(0);

        let transactions = self
            .wallet_repo
            .find_pending_transactions_for_party(query.actor_party_id, limit, offset)
            .await?;
        let total = self
            .wallet_repo
            .count_pending_transactions_for_party(query.actor_party_id)
            .await?;

        info!(
            party_id = %query.actor_party_id,
            total,
            "listed pending transaction approvals"
        );

        Ok(ListPendingApprovalsResult {
            transactions: transactions
                .into_iter()
                .map(TransactionResult::from)
                .collect(),
            total,
            limit,
            offset,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{FakePartyRepo, FakeWalletRepo};
    use domain::entities::{ApprovalDecision, Transaction, TransactionType};
    use rust_decimal::Decimal;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn lists_only_pending_transactions_awaiting_actor_party() {
        let party_repo = Arc::new(FakePartyRepo::default());
        let wallet_repo = Arc::new(FakeWalletRepo::default());

        let party_a = Uuid::now_v7();
        let party_b = Uuid::now_v7();
        let user_a = Uuid::now_v7();

        party_repo.parties.lock().unwrap().insert(
            party_a,
            domain::entities::Party::new(
                party_a,
                domain::entities::PartyType::Organization,
                domain::entities::DisplayName::new("Test A").unwrap(),
                domain::entities::Email::new("a@example.com").unwrap(),
            ),
        );
        party_repo
            .memberships
            .lock()
            .unwrap()
            .push(domain::entities::UserPartyMembership::new(
                Uuid::now_v7(),
                user_a,
                party_a,
                domain::entities::PartyMembershipRole::Owner,
            ));

        let txn = Transaction::new_pending(
            Uuid::now_v7(),
            Uuid::now_v7(),
            TransactionType::EscrowRelease,
            Some(party_a),
            Some(party_b),
            Decimal::from(50),
            2,
            vec![party_a, party_b],
            None,
            None,
            None,
        );
        wallet_repo.record_pending_transaction(&txn).await.unwrap();

        let uc = ListPendingApprovals::new(party_repo, wallet_repo.clone());
        let result = uc
            .execute(ListPendingApprovalsQuery {
                actor_user_id: user_a,
                actor_party_id: party_a,
                is_admin: false,
                limit: None,
                offset: None,
            })
            .await
            .unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.transactions[0].id, txn.id);

        // After party_a approves, the transaction should no longer be listed.
        wallet_repo
            .record_approval_and_finalise(
                &txn,
                &domain::entities::TransactionApproval::new(
                    Uuid::now_v7(),
                    txn.id,
                    party_a,
                    user_a,
                    ApprovalDecision::Approved,
                    None,
                ),
                &[],
            )
            .await
            .unwrap();

        let result = uc
            .execute(ListPendingApprovalsQuery {
                actor_user_id: user_a,
                actor_party_id: party_a,
                is_admin: false,
                limit: None,
                offset: None,
            })
            .await
            .unwrap();
        assert_eq!(result.total, 0);
    }

    #[tokio::test]
    async fn admin_can_list_pending_approvals_for_any_party() {
        let party_repo = Arc::new(FakePartyRepo::default());
        let wallet_repo = Arc::new(FakeWalletRepo::default());

        let party_a = Uuid::now_v7();
        let party_b = Uuid::now_v7();
        let admin_user_id = Uuid::now_v7();

        party_repo.parties.lock().unwrap().insert(
            party_a,
            domain::entities::Party::new(
                party_a,
                domain::entities::PartyType::Organization,
                domain::entities::DisplayName::new("Test A").unwrap(),
                domain::entities::Email::new("a@example.com").unwrap(),
            ),
        );

        let txn = Transaction::new_pending(
            Uuid::now_v7(),
            Uuid::now_v7(),
            TransactionType::EscrowRelease,
            Some(party_a),
            Some(party_b),
            Decimal::from(50),
            2,
            vec![party_a, party_b],
            None,
            None,
            None,
        );
        wallet_repo.record_pending_transaction(&txn).await.unwrap();

        let uc = ListPendingApprovals::new(party_repo, wallet_repo);
        let result = uc
            .execute(ListPendingApprovalsQuery {
                actor_user_id: admin_user_id,
                actor_party_id: party_a,
                is_admin: true,
                limit: None,
                offset: None,
            })
            .await
            .unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.transactions[0].id, txn.id);
    }
}
