use crate::errors::ApplicationError;
use crate::payments::dto::TransactionWithApprovalsResult;
use domain::repositories::{PartyRepository, WalletRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Get a single transaction and the approvals recorded against it.
#[derive(Clone)]
pub struct GetTransaction {
    party_repo: Arc<dyn PartyRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl GetTransaction {
    pub fn new(
        party_repo: Arc<dyn PartyRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
    ) -> Self {
        Self {
            party_repo,
            wallet_repo,
        }
    }

    #[instrument(skip(self), fields(transaction_id = %transaction_id, party_id = %actor_party_id))]
    pub async fn execute(
        &self,
        actor_user_id: Uuid,
        actor_party_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<TransactionWithApprovalsResult, ApplicationError> {
        if !self
            .party_repo
            .is_user_member_of_party(actor_user_id, actor_party_id)
            .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        let transaction = self
            .wallet_repo
            .find_transaction_by_id(transaction_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        if !transaction.involved_party_ids.contains(&actor_party_id)
            && transaction.from_party_id != Some(actor_party_id)
            && transaction.to_party_id != Some(actor_party_id)
        {
            return Err(ApplicationError::Forbidden);
        }

        let approvals = self
            .wallet_repo
            .find_approvals_for_transaction(transaction_id)
            .await?;

        info!(%transaction_id, approvals_count = approvals.len(), "fetched transaction with approvals");

        let approvals_required = transaction.approvals_required;
        let approvals_received = transaction.approvals_received;
        Ok(TransactionWithApprovalsResult {
            transaction: transaction.into(),
            approvals: approvals.into_iter().map(Into::into).collect(),
            approvals_required,
            approvals_received,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{FakePartyRepo, FakeWalletRepo};
    use domain::entities::{Transaction, TransactionType};
    use rust_decimal::Decimal;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn returns_transaction_and_approvals() {
        let party_repo = Arc::new(FakePartyRepo::default());
        let wallet_repo = Arc::new(FakeWalletRepo::default());

        let party_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        party_repo.parties.lock().unwrap().insert(
            party_id,
            domain::entities::Party::new(
                party_id,
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
                user_id,
                party_id,
                domain::entities::PartyMembershipRole::Owner,
            ));

        let txn = Transaction::new_pending(
            Uuid::now_v7(),
            Uuid::now_v7(),
            TransactionType::EscrowRelease,
            Some(party_id),
            Some(party_id),
            Decimal::from(10),
            1,
            vec![party_id],
            None,
            None,
            None,
        );
        wallet_repo.record_pending_transaction(&txn).await.unwrap();

        let uc = GetTransaction::new(party_repo, wallet_repo);
        let result = uc.execute(user_id, party_id, txn.id).await.unwrap();

        assert_eq!(result.transaction.id, txn.id);
        assert_eq!(result.approvals_required, 1);
        assert!(result.approvals.is_empty());
    }
}
