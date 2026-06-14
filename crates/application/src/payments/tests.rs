use crate::errors::ApplicationError;
use crate::payments::dto::{
    AdjustmentDirection, DeductFeeCommand, DepositPointsCommand, FeeSource, HoldEscrowCommand,
    ListTransactionsQuery, RecordAdjustmentCommand, ReleaseEscrowCommand, WithdrawPointsCommand,
};
use crate::payments::{
    CreateWallet, DeductFee, DepositPoints, GetDealWallet, GetWallet, HoldEscrow,
    ListDealTransactions, ListWalletTransactions, RecordAdjustment, ReleaseEscrow, WithdrawPoints,
};
use crate::test_helpers::{FakeDealRepo, FakePartyRepo, FakeWalletRepo};
use domain::entities::{
    DealParticipation, DealRole, DisplayName, Email, Party, PartyMembershipRole, PartyType,
    PlatformWallet, UserPartyMembership,
};
use domain::repositories::WalletRepository;
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

fn test_party(id: Uuid) -> Party {
    Party::new(
        id,
        PartyType::Organization,
        DisplayName::new("Test Party").unwrap(),
        Email::new("party@example.com").unwrap(),
    )
}

fn make_repos() -> (Arc<FakePartyRepo>, Arc<FakeDealRepo>, Arc<FakeWalletRepo>) {
    (
        Arc::new(FakePartyRepo::default()),
        Arc::new(FakeDealRepo::default()),
        Arc::new(FakeWalletRepo::default()),
    )
}

async fn seed_party_and_user(party_repo: &FakePartyRepo, user_id: Uuid, party_id: Uuid) {
    party_repo
        .parties
        .lock()
        .unwrap()
        .insert(party_id, test_party(party_id));
    party_repo
        .memberships
        .lock()
        .unwrap()
        .push(UserPartyMembership::new(
            Uuid::now_v7(),
            user_id,
            party_id,
            PartyMembershipRole::Owner,
        ));
}

async fn seed_deal_participation(deal_repo: &FakeDealRepo, deal_id: Uuid, party_id: Uuid) {
    deal_repo
        .participations
        .lock()
        .unwrap()
        .push(DealParticipation::new(
            Uuid::now_v7(),
            deal_id,
            party_id,
            DealRole::Supplier,
            true,
        ));
}

#[tokio::test]
async fn create_wallet_creates_container() {
    let party_repo = Arc::new(FakePartyRepo::default());
    let wallet_repo = Arc::new(FakeWalletRepo::default());
    let party_id = Uuid::now_v7();
    party_repo
        .parties
        .lock()
        .unwrap()
        .insert(party_id, test_party(party_id));

    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();

    let wallet = wallet_repo
        .find_by_party_id(party_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(wallet.party_id, party_id);
    assert_eq!(wallet.balance, Decimal::ZERO);
}

#[tokio::test]
async fn deposit_records_transaction_and_updates_balance() {
    let (party_repo, deal_repo, wallet_repo) = make_repos();
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    seed_party_and_user(&party_repo, user_id, party_id).await;
    seed_deal_participation(&deal_repo, deal_id, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();

    let uc = DepositPoints::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone());
    let result = uc
        .execute(DepositPointsCommand {
            actor_user_id: user_id,
            actor_party_id: party_id,
            deal_id,
            amount: Decimal::from(100),
            description: None,
            payment_method: None,
            external_reference: None,
        })
        .await
        .unwrap();

    assert_eq!(result.amount, Decimal::from(100));
    let wallet = wallet_repo
        .find_by_party_id(party_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(wallet.balance, Decimal::from(100));
}

#[tokio::test]
async fn deposit_rejects_non_member() {
    let (party_repo, deal_repo, wallet_repo) = make_repos();
    let party_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    party_repo
        .parties
        .lock()
        .unwrap()
        .insert(party_id, test_party(party_id));
    seed_deal_participation(&deal_repo, deal_id, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();

    let uc = DepositPoints::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone());
    let err = uc
        .execute(DepositPointsCommand {
            actor_user_id: Uuid::now_v7(),
            actor_party_id: party_id,
            deal_id,
            amount: Decimal::from(10),
            description: None,
            payment_method: None,
            external_reference: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Forbidden));
}

#[tokio::test]
async fn deposit_rejects_non_participant_deal() {
    let (party_repo, deal_repo, wallet_repo) = make_repos();
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    seed_party_and_user(&party_repo, user_id, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();

    let uc = DepositPoints::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone());
    let err = uc
        .execute(DepositPointsCommand {
            actor_user_id: user_id,
            actor_party_id: party_id,
            deal_id,
            amount: Decimal::from(10),
            description: None,
            payment_method: None,
            external_reference: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::DealAccessDenied));
}

#[tokio::test]
async fn withdraw_requires_sufficient_balance() {
    let (party_repo, deal_repo, wallet_repo) = make_repos();
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    seed_party_and_user(&party_repo, user_id, party_id).await;
    seed_deal_participation(&deal_repo, deal_id, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();

    let withdraw = WithdrawPoints::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone());
    let err = withdraw
        .execute(WithdrawPointsCommand {
            actor_user_id: user_id,
            actor_party_id: party_id,
            deal_id,
            amount: Decimal::from(50),
            description: None,
            payment_method: None,
            external_reference: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn withdraw_reduces_balance() {
    let (party_repo, deal_repo, wallet_repo) = make_repos();
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    seed_party_and_user(&party_repo, user_id, party_id).await;
    seed_deal_participation(&deal_repo, deal_id, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();

    wallet_repo.wallets.lock().unwrap().insert(party_id, {
        let mut w = PlatformWallet::new(Uuid::now_v7(), party_id);
        w.deposit(Decimal::from(200)).unwrap();
        w
    });

    let withdraw = WithdrawPoints::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone());
    withdraw
        .execute(WithdrawPointsCommand {
            actor_user_id: user_id,
            actor_party_id: party_id,
            deal_id,
            amount: Decimal::from(75),
            description: None,
            payment_method: None,
            external_reference: None,
        })
        .await
        .unwrap();

    let wallet = wallet_repo
        .find_by_party_id(party_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(wallet.balance, Decimal::from(125));
}

#[tokio::test]
async fn hold_and_release_escrow_update_balances() {
    let (party_repo, deal_repo, wallet_repo) = make_repos();
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    seed_party_and_user(&party_repo, user_id, party_id).await;
    seed_deal_participation(&deal_repo, deal_id, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();
    wallet_repo.wallets.lock().unwrap().insert(party_id, {
        let mut w = PlatformWallet::new(Uuid::now_v7(), party_id);
        w.deposit(Decimal::from(500)).unwrap();
        w
    });

    let hold = HoldEscrow::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone());
    hold.execute(HoldEscrowCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        deal_id,
        amount: Decimal::from(300),
        description: None,
        payment_method: None,
        external_reference: None,
    })
    .await
    .unwrap();

    let wallet = wallet_repo
        .find_by_party_id(party_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(wallet.balance, Decimal::from(200));
    assert_eq!(wallet.escrow_balance, Decimal::from(300));

    let release = ReleaseEscrow::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone());
    release
        .execute(ReleaseEscrowCommand {
            actor_user_id: user_id,
            actor_party_id: party_id,
            deal_id,
            amount: Decimal::from(100),
            description: None,
            payment_method: None,
            external_reference: None,
        })
        .await
        .unwrap();

    let wallet = wallet_repo
        .find_by_party_id(party_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(wallet.balance, Decimal::from(300));
    assert_eq!(wallet.escrow_balance, Decimal::from(200));
}

#[tokio::test]
async fn deduct_fee_from_balance_and_escrow() {
    let (party_repo, deal_repo, wallet_repo) = make_repos();
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    seed_party_and_user(&party_repo, user_id, party_id).await;
    seed_deal_participation(&deal_repo, deal_id, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();
    wallet_repo.wallets.lock().unwrap().insert(party_id, {
        let mut w = PlatformWallet::new(Uuid::now_v7(), party_id);
        w.deposit(Decimal::from(100)).unwrap();
        w.hold_escrow(Decimal::from(60)).unwrap();
        w
    });

    let fee = DeductFee::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone());
    fee.execute(DeductFeeCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        deal_id,
        amount: Decimal::from(10),
        source: FeeSource::Balance,
        description: None,
        payment_method: None,
        external_reference: None,
    })
    .await
    .unwrap();

    fee.execute(DeductFeeCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        deal_id,
        amount: Decimal::from(20),
        source: FeeSource::Escrow,
        description: None,
        payment_method: None,
        external_reference: None,
    })
    .await
    .unwrap();

    let wallet = wallet_repo
        .find_by_party_id(party_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(wallet.balance, Decimal::from(30));
    assert_eq!(wallet.escrow_balance, Decimal::from(40));
}

#[tokio::test]
async fn record_adjustment_credits_and_debits() {
    let (party_repo, deal_repo, wallet_repo) = make_repos();
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    seed_party_and_user(&party_repo, user_id, party_id).await;
    seed_deal_participation(&deal_repo, deal_id, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();
    wallet_repo.wallets.lock().unwrap().insert(party_id, {
        let mut w = PlatformWallet::new(Uuid::now_v7(), party_id);
        w.deposit(Decimal::from(100)).unwrap();
        w
    });

    let adjust = RecordAdjustment::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone());
    adjust
        .execute(RecordAdjustmentCommand {
            actor_user_id: user_id,
            actor_party_id: party_id,
            deal_id,
            amount: Decimal::from(25),
            direction: AdjustmentDirection::Credit,
            description: None,
            payment_method: None,
            external_reference: None,
        })
        .await
        .unwrap();

    adjust
        .execute(RecordAdjustmentCommand {
            actor_user_id: user_id,
            actor_party_id: party_id,
            deal_id,
            amount: Decimal::from(10),
            direction: AdjustmentDirection::Debit,
            description: None,
            payment_method: None,
            external_reference: None,
        })
        .await
        .unwrap();

    let wallet = wallet_repo
        .find_by_party_id(party_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(wallet.balance, Decimal::from(115));
}

#[tokio::test]
async fn get_wallet_returns_container() {
    let party_repo = Arc::new(FakePartyRepo::default());
    let wallet_repo = Arc::new(FakeWalletRepo::default());
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    seed_party_and_user(&party_repo, user_id, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();

    let result = GetWallet::new(party_repo, wallet_repo)
        .execute(user_id, party_id)
        .await
        .unwrap();

    assert_eq!(result.party_id, party_id);
}

#[tokio::test]
async fn get_deal_wallet_computes_sub_wallet() {
    let (party_repo, deal_repo, wallet_repo) = make_repos();
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    seed_party_and_user(&party_repo, user_id, party_id).await;
    seed_deal_participation(&deal_repo, deal_id, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();

    DepositPoints::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone())
        .execute(DepositPointsCommand {
            actor_user_id: user_id,
            actor_party_id: party_id,
            deal_id,
            amount: Decimal::from(200),
            description: None,
            payment_method: None,
            external_reference: None,
        })
        .await
        .unwrap();

    let result = GetDealWallet::new(party_repo, deal_repo, wallet_repo)
        .execute(user_id, party_id, deal_id)
        .await
        .unwrap();

    assert_eq!(result.deal_id, deal_id);
    assert_eq!(result.deposited, Decimal::from(200));
}

#[tokio::test]
async fn list_wallet_transactions_paginates_and_filters() {
    let (party_repo, deal_repo, wallet_repo) = make_repos();
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    seed_party_and_user(&party_repo, user_id, party_id).await;
    seed_deal_participation(&deal_repo, deal_id, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();

    for i in 1..=3 {
        DepositPoints::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone())
            .execute(DepositPointsCommand {
                actor_user_id: user_id,
                actor_party_id: party_id,
                deal_id,
                amount: Decimal::from(i * 10),
                description: None,
                payment_method: None,
                external_reference: None,
            })
            .await
            .unwrap();
    }

    let result = ListWalletTransactions::new(party_repo.clone(), wallet_repo.clone())
        .execute(
            user_id,
            party_id,
            ListTransactionsQuery {
                limit: Some(2),
                offset: Some(0),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(result.transactions.len(), 2);
    assert_eq!(result.total, 3);

    let filtered = ListWalletTransactions::new(party_repo, wallet_repo)
        .execute(
            user_id,
            party_id,
            ListTransactionsQuery {
                transaction_type: Some("DEPOSIT".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(filtered.total, 3);
}

#[tokio::test]
async fn list_deal_transactions_filters_by_deal() {
    let (party_repo, deal_repo, wallet_repo) = make_repos();
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let deal_a = Uuid::now_v7();
    let deal_b = Uuid::now_v7();
    seed_party_and_user(&party_repo, user_id, party_id).await;
    seed_deal_participation(&deal_repo, deal_a, party_id).await;
    seed_deal_participation(&deal_repo, deal_b, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();

    for deal_id in [deal_a, deal_b] {
        DepositPoints::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone())
            .execute(DepositPointsCommand {
                actor_user_id: user_id,
                actor_party_id: party_id,
                deal_id,
                amount: Decimal::from(50),
                description: None,
                payment_method: None,
                external_reference: None,
            })
            .await
            .unwrap();
    }

    let result =
        ListDealTransactions::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone())
            .execute(user_id, party_id, deal_a, ListTransactionsQuery::default())
            .await
            .unwrap();

    assert_eq!(result.total, 1);
    assert_eq!(result.transactions[0].deal_id, deal_a);
}

#[tokio::test]
async fn list_deal_transactions_rejects_non_participant_deal() {
    let (party_repo, deal_repo, wallet_repo) = make_repos();
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    seed_party_and_user(&party_repo, user_id, party_id).await;
    CreateWallet::new(wallet_repo.clone())
        .execute(party_id)
        .await
        .unwrap();

    let err = ListDealTransactions::new(party_repo, deal_repo, wallet_repo)
        .execute(user_id, party_id, deal_id, ListTransactionsQuery::default())
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::DealAccessDenied));
}
