use crate::errors::ApplicationError;
use crate::payments::dto::{ApproveTransactionCommand, TransactionResult};
use domain::entities::{
    ApprovalDecision, PlatformWallet, Transaction, TransactionApproval, TransactionStatus,
    TransactionType,
};
use domain::repositories::{PartyRepository, WalletRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Approve or reject a pending transaction and finalise it when all required
/// parties have approved.
#[derive(Clone)]
pub struct ApproveTransaction {
    party_repo: Arc<dyn PartyRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl ApproveTransaction {
    pub fn new(
        party_repo: Arc<dyn PartyRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
    ) -> Self {
        Self {
            party_repo,
            wallet_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(transaction_id = %cmd.transaction_id, party_id = %cmd.actor_party_id))]
    pub async fn execute(
        &self,
        cmd: ApproveTransactionCommand,
    ) -> Result<TransactionResult, ApplicationError> {
        if !self
            .party_repo
            .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
            .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        let transaction = self
            .wallet_repo
            .find_transaction_by_id(cmd.transaction_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        if transaction.status != TransactionStatus::Pending || !transaction.requires_approval {
            return Err(ApplicationError::Validation(vec![
                "transaction is not pending approval".to_string(),
            ]));
        }

        if !transaction.involved_party_ids.contains(&cmd.actor_party_id) {
            return Err(ApplicationError::Forbidden);
        }

        let existing_approvals = self
            .wallet_repo
            .find_approvals_for_transaction(transaction.id)
            .await?;
        if existing_approvals
            .iter()
            .any(|a| a.party_id == cmd.actor_party_id)
        {
            return Err(ApplicationError::Validation(vec![
                "party has already recorded a decision for this transaction".to_string(),
            ]));
        }

        let will_finalize = cmd.decision == ApprovalDecision::Approved
            && existing_approvals.len() as i32 + 1 >= transaction.approvals_required;

        let approval = TransactionApproval::new(
            Uuid::now_v7(),
            transaction.id,
            cmd.actor_party_id,
            cmd.actor_user_id,
            cmd.decision,
            cmd.comment.clone(),
        );

        let wallet_mutations = self
            .compute_wallet_mutations(&transaction, &cmd.decision, will_finalize)
            .await?;

        self.wallet_repo
            .record_approval_and_finalise(&transaction, &approval, &wallet_mutations)
            .await?;

        let updated = self
            .wallet_repo
            .find_transaction_by_id(transaction.id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        info!(
            transaction_id = %transaction.id,
            decision = %cmd.decision.as_str(),
            status = %updated.status.as_str(),
            "recorded transaction decision"
        );

        Ok(updated.into())
    }

    async fn compute_wallet_mutations(
        &self,
        transaction: &Transaction,
        decision: &ApprovalDecision,
        finalize: bool,
    ) -> Result<Vec<(Uuid, PlatformWallet)>, ApplicationError> {
        match decision {
            ApprovalDecision::Rejected => self.rejection_mutations(transaction).await,
            ApprovalDecision::Approved if finalize => self.approval_mutations(transaction).await,
            ApprovalDecision::Approved => Ok(vec![]),
        }
    }

    async fn rejection_mutations(
        &self,
        transaction: &Transaction,
    ) -> Result<Vec<(Uuid, PlatformWallet)>, ApplicationError> {
        // For transaction types that hold available balance while pending,
        // return the held amount to the source wallet on rejection.
        match transaction.transaction_type {
            TransactionType::Withdrawal | TransactionType::Fee => {
                if let Some(party_id) = transaction.from_party_id {
                    let mut wallet = self.require_wallet(party_id).await?;
                    wallet.release_pending(transaction.amount)?;
                    return Ok(vec![(party_id, wallet)]);
                }
            }
            TransactionType::EscrowHold => {
                if let Some(party_id) = transaction.from_party_id {
                    let mut wallet = self.require_wallet(party_id).await?;
                    wallet.release_pending(transaction.amount)?;
                    return Ok(vec![(party_id, wallet)]);
                }
            }
            _ => {}
        }
        Ok(vec![])
    }

    async fn approval_mutations(
        &self,
        transaction: &Transaction,
    ) -> Result<Vec<(Uuid, PlatformWallet)>, ApplicationError> {
        use TransactionType::*;

        match transaction.transaction_type {
            Deposit => {
                let party_id = transaction.to_party_id.ok_or_else(|| {
                    ApplicationError::Validation(vec![
                        "deposit transaction must have a destination party".to_string(),
                    ])
                })?;
                let mut wallet = self.require_wallet(party_id).await?;
                wallet.deposit(transaction.amount)?;
                Ok(vec![(party_id, wallet)])
            }
            Withdrawal => {
                let party_id = transaction.from_party_id.ok_or_else(|| {
                    ApplicationError::Validation(vec![
                        "withdrawal transaction must have a source party".to_string(),
                    ])
                })?;
                let mut wallet = self.require_wallet(party_id).await?;
                wallet.withdraw(transaction.amount)?;
                Ok(vec![(party_id, wallet)])
            }
            EscrowHold => {
                let party_id = transaction.from_party_id.ok_or_else(|| {
                    ApplicationError::Validation(vec![
                        "escrow hold transaction must have a source party".to_string(),
                    ])
                })?;
                let mut wallet = self.require_wallet(party_id).await?;
                wallet.commit_pending_to_escrow(transaction.amount)?;
                Ok(vec![(party_id, wallet)])
            }
            EscrowRelease => {
                let from = transaction.from_party_id.ok_or_else(|| {
                    ApplicationError::Validation(vec![
                        "escrow release transaction must have a source party".to_string(),
                    ])
                })?;
                let to = transaction.to_party_id.ok_or_else(|| {
                    ApplicationError::Validation(vec![
                        "escrow release transaction must have a destination party".to_string(),
                    ])
                })?;

                if from == to {
                    let mut wallet = self.require_wallet(from).await?;
                    wallet.release_escrow_to_self(transaction.amount)?;
                    return Ok(vec![(from, wallet)]);
                }

                let mut source = self.require_wallet(from).await?;
                let mut recipient = self.require_wallet(to).await?;
                source.debit_escrow(transaction.amount)?;
                recipient.credit_balance(transaction.amount)?;
                Ok(vec![(from, source), (to, recipient)])
            }
            Fee => {
                let party_id = transaction.from_party_id.ok_or_else(|| {
                    ApplicationError::Validation(vec![
                        "fee transaction must have a source party".to_string()
                    ])
                })?;
                let mut wallet = self.require_wallet(party_id).await?;
                wallet.commit_pending_to_balance(transaction.amount)?;
                wallet.deduct_fee_from_balance(transaction.amount)?;
                Ok(vec![(party_id, wallet)])
            }
            Adjustment => {
                let mut mutations = Vec::new();
                if let Some(from) = transaction.from_party_id {
                    let mut wallet = self.require_wallet(from).await?;
                    wallet.commit_pending_to_balance(transaction.amount)?;
                    wallet.withdraw(transaction.amount)?;
                    mutations.push((from, wallet));
                }
                if let Some(to) = transaction.to_party_id {
                    if Some(to) != transaction.from_party_id {
                        let mut wallet = self.require_wallet(to).await?;
                        wallet.deposit(transaction.amount)?;
                        mutations.push((to, wallet));
                    }
                }
                Ok(mutations)
            }
        }
    }

    async fn require_wallet(&self, party_id: Uuid) -> Result<PlatformWallet, ApplicationError> {
        self.wallet_repo
            .find_by_party_id(party_id)
            .await?
            .ok_or(ApplicationError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{FakePartyRepo, FakeWalletRepo};
    use domain::entities::{Transaction, TransactionType};
    use rust_decimal::Decimal;
    use std::sync::Arc;

    fn seed_party_user(party_repo: &FakePartyRepo, user_id: Uuid, party_id: Uuid) {
        use domain::entities::{DisplayName, Email, Party, PartyType, UserPartyMembership};
        party_repo.parties.lock().unwrap().insert(
            party_id,
            Party::new(
                party_id,
                PartyType::Organization,
                DisplayName::new("Test Party").unwrap(),
                Email::new("party@example.com").unwrap(),
            ),
        );
        party_repo
            .memberships
            .lock()
            .unwrap()
            .push(UserPartyMembership::new(
                Uuid::now_v7(),
                user_id,
                party_id,
                domain::entities::PartyMembershipRole::Owner,
            ));
    }

    fn seed_wallet(
        wallet_repo: &FakeWalletRepo,
        party_id: Uuid,
        balance: Decimal,
        escrow: Decimal,
    ) {
        let mut wallet = PlatformWallet::new(Uuid::now_v7(), party_id);
        let total = balance + escrow;
        if total > Decimal::ZERO {
            wallet.deposit(total).unwrap();
        }
        if escrow > Decimal::ZERO {
            wallet.hold_escrow(escrow).unwrap();
        }
        wallet_repo.wallets.lock().unwrap().insert(party_id, wallet);
    }

    fn pending_escrow_release(
        deal_id: Uuid,
        from: Uuid,
        to: Uuid,
        third: Uuid,
        amount: Decimal,
    ) -> Transaction {
        Transaction::new_pending(
            Uuid::now_v7(),
            deal_id,
            TransactionType::EscrowRelease,
            Some(from),
            Some(to),
            amount,
            3,
            vec![from, to, third],
            Some("milestone release".to_string()),
            None,
            None,
        )
    }

    #[tokio::test]
    async fn approve_escrow_release_requires_all_three_parties() {
        let party_repo = Arc::new(FakePartyRepo::default());
        let wallet_repo = Arc::new(FakeWalletRepo::default());

        let deal_id = Uuid::now_v7();
        let consumer = Uuid::now_v7();
        let supplier = Uuid::now_v7();
        let enhancer = Uuid::now_v7();

        for (user_id, party_id) in [
            (Uuid::now_v7(), consumer),
            (Uuid::now_v7(), supplier),
            (Uuid::now_v7(), enhancer),
        ] {
            seed_party_user(&party_repo, user_id, party_id);
        }
        seed_wallet(&wallet_repo, consumer, Decimal::from(0), Decimal::from(300));
        seed_wallet(&wallet_repo, supplier, Decimal::from(0), Decimal::from(0));
        seed_wallet(&wallet_repo, enhancer, Decimal::from(0), Decimal::from(0));

        let txn = pending_escrow_release(deal_id, consumer, supplier, enhancer, Decimal::from(100));
        wallet_repo.record_pending_transaction(&txn).await.unwrap();

        let uc = ApproveTransaction::new(party_repo.clone(), wallet_repo.clone());

        // First approval keeps it pending.
        let first_user = party_repo.memberships.lock().unwrap()[0].user_id;
        let result = uc
            .execute(ApproveTransactionCommand {
                actor_user_id: first_user,
                actor_party_id: consumer,
                transaction_id: txn.id,
                decision: ApprovalDecision::Approved,
                comment: None,
            })
            .await
            .unwrap();
        assert_eq!(result.status, "PENDING");

        // Non-involved party cannot approve.
        let outsider_user = Uuid::now_v7();
        seed_party_user(&party_repo, outsider_user, Uuid::now_v7());
        let err = uc
            .execute(ApproveTransactionCommand {
                actor_user_id: outsider_user,
                actor_party_id: Uuid::now_v7(),
                transaction_id: txn.id,
                decision: ApprovalDecision::Approved,
                comment: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ApplicationError::Forbidden));

        // Second approval still pending.
        let second_user = party_repo.memberships.lock().unwrap()[1].user_id;
        let result = uc
            .execute(ApproveTransactionCommand {
                actor_user_id: second_user,
                actor_party_id: supplier,
                transaction_id: txn.id,
                decision: ApprovalDecision::Approved,
                comment: None,
            })
            .await
            .unwrap();
        assert_eq!(result.status, "PENDING");

        // Third approval finalises.
        let third_user = party_repo.memberships.lock().unwrap()[2].user_id;
        let result = uc
            .execute(ApproveTransactionCommand {
                actor_user_id: third_user,
                actor_party_id: enhancer,
                transaction_id: txn.id,
                decision: ApprovalDecision::Approved,
                comment: None,
            })
            .await
            .unwrap();
        assert_eq!(result.status, "VERIFIED");

        let consumer_wallet = wallet_repo
            .find_by_party_id(consumer)
            .await
            .unwrap()
            .unwrap();
        let supplier_wallet = wallet_repo
            .find_by_party_id(supplier)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(consumer_wallet.escrow_balance, Decimal::from(200));
        assert_eq!(supplier_wallet.balance, Decimal::from(100));
    }

    #[tokio::test]
    async fn rejection_marks_transaction_rejected_and_preserves_escrow() {
        let party_repo = Arc::new(FakePartyRepo::default());
        let wallet_repo = Arc::new(FakeWalletRepo::default());

        let deal_id = Uuid::now_v7();
        let consumer = Uuid::now_v7();
        let supplier = Uuid::now_v7();
        let enhancer = Uuid::now_v7();

        for (user_id, party_id) in [
            (Uuid::now_v7(), consumer),
            (Uuid::now_v7(), supplier),
            (Uuid::now_v7(), enhancer),
        ] {
            seed_party_user(&party_repo, user_id, party_id);
        }
        seed_wallet(&wallet_repo, consumer, Decimal::from(0), Decimal::from(300));

        let txn = pending_escrow_release(deal_id, consumer, supplier, enhancer, Decimal::from(100));
        wallet_repo.record_pending_transaction(&txn).await.unwrap();

        let uc = ApproveTransaction::new(party_repo.clone(), wallet_repo.clone());
        let user_id = party_repo.memberships.lock().unwrap()[0].user_id;

        let result = uc
            .execute(ApproveTransactionCommand {
                actor_user_id: user_id,
                actor_party_id: consumer,
                transaction_id: txn.id,
                decision: ApprovalDecision::Rejected,
                comment: Some("dispute".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(result.status, "REJECTED");
        let wallet = wallet_repo
            .find_by_party_id(consumer)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(wallet.escrow_balance, Decimal::from(300));
    }

    #[tokio::test]
    async fn cannot_approve_twice() {
        let party_repo = Arc::new(FakePartyRepo::default());
        let wallet_repo = Arc::new(FakeWalletRepo::default());

        let user_id = Uuid::now_v7();
        let party_id = Uuid::now_v7();
        seed_party_user(&party_repo, user_id, party_id);
        seed_wallet(&wallet_repo, party_id, Decimal::from(0), Decimal::from(50));

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

        let uc = ApproveTransaction::new(party_repo.clone(), wallet_repo.clone());
        uc.execute(ApproveTransactionCommand {
            actor_user_id: user_id,
            actor_party_id: party_id,
            transaction_id: txn.id,
            decision: ApprovalDecision::Approved,
            comment: None,
        })
        .await
        .unwrap();

        let err = uc
            .execute(ApproveTransactionCommand {
                actor_user_id: user_id,
                actor_party_id: party_id,
                transaction_id: txn.id,
                decision: ApprovalDecision::Approved,
                comment: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }
}
