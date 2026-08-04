use crate::deals::settlement_saga::SettlementSaga;
use crate::errors::ApplicationError;
use crate::parties::dto::CreatePartyCommand;
use crate::parties::CreateParty;
use crate::test_helpers::{FakeDealRepo, FakePartyRepo, FakeWalletRepo};
use domain::entities::{
    Deal, DealParticipation, DealRole, DealStatus, DealTitle, DistributionModel, PartyType,
    PlatformWallet, ValueDistribution,
};
use domain::repositories::{DealAggregate, DealRepository, WalletRepository};
use rust_decimal::Decimal;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

fn actor_user_id() -> Uuid {
    Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()
}

async fn create_party(repo: &Arc<FakePartyRepo>, display_name: &str, roles: Vec<DealRole>) -> Uuid {
    CreateParty::new(repo.clone())
        .execute(CreatePartyCommand {
            actor_user_id: actor_user_id(),
            party_type: PartyType::Organization,
            display_name: display_name.to_string(),
            email: format!("{}@example.com", display_name.to_lowercase()),
            phone: None,
            tax_id: None,
            primary_domain_id: None,
            latitude: None,
            longitude: None,
            service_radius_km: None,
            roles,
        })
        .await
        .unwrap()
        .id
}

fn distribution(deal_id: Uuid) -> ValueDistribution {
    ValueDistribution {
        id: Uuid::now_v7(),
        deal_id,
        total_value: Decimal::from(10000),
        currency: "POINTS".to_string(),
        distribution_model: DistributionModel::FixedPrice,
        supplier_share_percentage: Decimal::from(60),
        supplier_share_amount: Decimal::from(6000),
        consumer_cost_percentage: Decimal::from(100),
        consumer_cost_amount: Decimal::from(10000),
        enhancer_share_percentage: Decimal::from(30),
        enhancer_share_amount: Decimal::from(3000),
        platform_fee_percentage: Decimal::from(10),
        platform_fee_amount: Decimal::from(1000),
        payment_schedule: vec![],
        win_win_win_score: None,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    }
}

async fn executing_deal_fixture() -> (
    Arc<FakePartyRepo>,
    Arc<FakeDealRepo>,
    Arc<FakeWalletRepo>,
    Uuid,
    Uuid,
    Uuid,
    Uuid,
) {
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());
    let wallet_repo = Arc::new(FakeWalletRepo::default());

    let supplier = create_party(&party_repo, "Supplier", vec![DealRole::Supplier]).await;
    let consumer = create_party(&party_repo, "Consumer", vec![DealRole::Consumer]).await;
    let enhancer = create_party(&party_repo, "Enhancer", vec![DealRole::Enhancer]).await;

    let category_id = Uuid::parse_str("55555555-5555-5555-5555-555555555555").unwrap();
    let deal_id = Uuid::now_v7();
    let mut deal = Deal::new(
        deal_id,
        "DL-2026-SETTLE".to_string(),
        DealTitle::new("Settlement Test Deal").unwrap(),
        category_id,
        supplier,
        DealRole::Supplier,
    );
    deal.deal_status = DealStatus::Executing;

    deal_repo
        .create(&DealAggregate {
            deal,
            participations: vec![
                DealParticipation::new(Uuid::now_v7(), deal_id, supplier, DealRole::Supplier, true),
                DealParticipation::new(
                    Uuid::now_v7(),
                    deal_id,
                    consumer,
                    DealRole::Consumer,
                    false,
                ),
                DealParticipation::new(
                    Uuid::now_v7(),
                    deal_id,
                    enhancer,
                    DealRole::Enhancer,
                    false,
                ),
            ],
        })
        .await
        .unwrap();

    deal_repo
        .set_value_distribution(&distribution(deal_id))
        .await
        .unwrap();

    (
        party_repo,
        deal_repo,
        wallet_repo,
        deal_id,
        supplier,
        consumer,
        enhancer,
    )
}

async fn fund_consumer_escrow(wallet_repo: &Arc<FakeWalletRepo>, consumer: Uuid, amount: Decimal) {
    let mut wallet = PlatformWallet::new(Uuid::now_v7(), consumer);
    wallet.deposit(amount).unwrap();
    wallet.hold_escrow(amount).unwrap();
    wallet_repo.create(&wallet).await.unwrap();
}

async fn deal_repo_with_participations(
    source: &Arc<FakeDealRepo>,
    deal_id: Uuid,
    participations: Vec<DealParticipation>,
) -> Arc<FakeDealRepo> {
    let repo = Arc::new(FakeDealRepo::default());
    let deal = source.find_by_id(deal_id).await.unwrap().unwrap();
    let distribution = source
        .find_value_distribution_by_deal(deal_id)
        .await
        .unwrap()
        .unwrap();
    repo.create(&DealAggregate {
        deal,
        participations,
    })
    .await
    .unwrap();
    repo.set_value_distribution(&distribution).await.unwrap();
    repo
}

#[tokio::test]
async fn settlement_fails_when_deal_not_executing() {
    let (_party_repo, deal_repo, wallet_repo, deal_id, supplier, consumer, enhancer) =
        executing_deal_fixture().await;

    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Committed;
    deal_repo.update(&deal).await.unwrap();

    fund_consumer_escrow(&wallet_repo, consumer, Decimal::from(10000)).await;

    let saga = SettlementSaga::new(deal_repo.clone(), wallet_repo.clone());
    let err = saga.execute(deal_id, actor_user_id()).await.unwrap_err();

    assert!(
        matches!(err, ApplicationError::InvalidStateTransition { .. }),
        "expected InvalidStateTransition, got {:?}",
        err
    );

    let supplier_wallet = wallet_repo.find_by_party_id(supplier).await.unwrap();
    let enhancer_wallet = wallet_repo.find_by_party_id(enhancer).await.unwrap();
    assert!(supplier_wallet.is_none());
    assert!(enhancer_wallet.is_none());
}

#[tokio::test]
async fn settlement_fails_when_value_distribution_missing() {
    let (_party_repo, deal_repo, wallet_repo, deal_id, supplier, consumer, enhancer) =
        executing_deal_fixture().await;

    // Remove the value distribution by replacing the repo with a fresh one that still
    // has the deal but no distribution.
    let clean_deal_repo = Arc::new(FakeDealRepo::default());
    let aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await
        .unwrap()
        .unwrap();
    clean_deal_repo.create(&aggregate).await.unwrap();

    fund_consumer_escrow(&wallet_repo, consumer, Decimal::from(10000)).await;

    let saga = SettlementSaga::new(clean_deal_repo, wallet_repo.clone());
    let err = saga.execute(deal_id, actor_user_id()).await.unwrap_err();

    assert!(
        matches!(err, ApplicationError::SettlementFailed { .. }),
        "expected SettlementFailed, got {:?}",
        err
    );
    let supplier_wallet = wallet_repo.find_by_party_id(supplier).await.unwrap();
    let enhancer_wallet = wallet_repo.find_by_party_id(enhancer).await.unwrap();
    assert!(supplier_wallet.is_none());
    assert!(enhancer_wallet.is_none());
}

#[tokio::test]
async fn settlement_fails_when_consumer_participation_missing() {
    let (_party_repo, deal_repo, wallet_repo, deal_id, supplier, _consumer, enhancer) =
        executing_deal_fixture().await;

    let repo = deal_repo_with_participations(
        &deal_repo,
        deal_id,
        vec![
            DealParticipation::new(Uuid::now_v7(), deal_id, supplier, DealRole::Supplier, true),
            DealParticipation::new(Uuid::now_v7(), deal_id, enhancer, DealRole::Enhancer, false),
        ],
    )
    .await;

    let saga = SettlementSaga::new(repo, wallet_repo.clone());
    let err = saga.execute(deal_id, actor_user_id()).await.unwrap_err();

    assert!(
        matches!(err, ApplicationError::SettlementFailed { .. }),
        "expected SettlementFailed, got {:?}",
        err
    );
}

#[tokio::test]
async fn settlement_fails_when_supplier_participation_missing() {
    let (_party_repo, deal_repo, wallet_repo, deal_id, supplier, consumer, _enhancer) =
        executing_deal_fixture().await;

    let repo = deal_repo_with_participations(
        &deal_repo,
        deal_id,
        vec![
            DealParticipation::new(Uuid::now_v7(), deal_id, supplier, DealRole::Supplier, true),
            DealParticipation::new(Uuid::now_v7(), deal_id, consumer, DealRole::Consumer, false),
        ],
    )
    .await;

    fund_consumer_escrow(&wallet_repo, consumer, Decimal::from(10000)).await;

    let saga = SettlementSaga::new(repo, wallet_repo.clone());
    let err = saga.execute(deal_id, actor_user_id()).await.unwrap_err();

    assert!(
        matches!(err, ApplicationError::SettlementFailed { .. }),
        "expected SettlementFailed, got {:?}",
        err
    );
}

#[tokio::test]
async fn settlement_fails_when_enhancer_participation_missing() {
    let (_party_repo, deal_repo, wallet_repo, deal_id, supplier, consumer, _enhancer) =
        executing_deal_fixture().await;

    let repo = deal_repo_with_participations(
        &deal_repo,
        deal_id,
        vec![
            DealParticipation::new(Uuid::now_v7(), deal_id, supplier, DealRole::Supplier, true),
            DealParticipation::new(Uuid::now_v7(), deal_id, consumer, DealRole::Consumer, false),
        ],
    )
    .await;

    fund_consumer_escrow(&wallet_repo, consumer, Decimal::from(10000)).await;

    let saga = SettlementSaga::new(repo, wallet_repo.clone());
    let err = saga.execute(deal_id, actor_user_id()).await.unwrap_err();

    assert!(
        matches!(err, ApplicationError::SettlementFailed { .. }),
        "expected SettlementFailed, got {:?}",
        err
    );
}

#[tokio::test]
async fn settlement_fails_when_consumer_escrow_insufficient() {
    let (_party_repo, deal_repo, wallet_repo, deal_id, _supplier, consumer, _enhancer) =
        executing_deal_fixture().await;

    fund_consumer_escrow(&wallet_repo, consumer, Decimal::from(5000)).await;

    let saga = SettlementSaga::new(deal_repo.clone(), wallet_repo.clone());
    let err = saga.execute(deal_id, actor_user_id()).await.unwrap_err();

    assert!(
        matches!(err, ApplicationError::SettlementFailed { .. }),
        "expected SettlementFailed, got {:?}",
        err
    );
}

#[tokio::test]
async fn settlement_creates_missing_supplier_and_enhancer_wallets() {
    let (_party_repo, deal_repo, wallet_repo, deal_id, supplier, consumer, enhancer) =
        executing_deal_fixture().await;

    fund_consumer_escrow(&wallet_repo, consumer, Decimal::from(10000)).await;

    assert!(wallet_repo
        .find_by_party_id(supplier)
        .await
        .unwrap()
        .is_none());
    assert!(wallet_repo
        .find_by_party_id(enhancer)
        .await
        .unwrap()
        .is_none());

    let saga = SettlementSaga::new(deal_repo.clone(), wallet_repo.clone());
    saga.execute(deal_id, actor_user_id()).await.unwrap();

    let supplier_wallet = wallet_repo
        .find_by_party_id(supplier)
        .await
        .unwrap()
        .unwrap();
    let enhancer_wallet = wallet_repo
        .find_by_party_id(enhancer)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(supplier_wallet.balance, Decimal::from(6000));
    assert_eq!(enhancer_wallet.balance, Decimal::from(3000));
}

#[tokio::test]
async fn settlement_succeeds_with_zero_platform_fee() {
    let (_party_repo, deal_repo, wallet_repo, deal_id, supplier, consumer, enhancer) =
        executing_deal_fixture().await;

    let mut distribution = distribution(deal_id);
    distribution.platform_fee_amount = Decimal::ZERO;
    distribution.platform_fee_percentage = Decimal::ZERO;
    distribution.supplier_share_amount = Decimal::from(7000);
    distribution.enhancer_share_amount = Decimal::from(3000);
    deal_repo
        .set_value_distribution(&distribution)
        .await
        .unwrap();

    fund_consumer_escrow(&wallet_repo, consumer, Decimal::from(10000)).await;

    let saga = SettlementSaga::new(deal_repo.clone(), wallet_repo.clone());
    let result = saga.execute(deal_id, actor_user_id()).await.unwrap();

    // No fee transaction, two escrow releases.
    assert_eq!(result.transaction_ids.len(), 2);

    let consumer_wallet = wallet_repo
        .find_by_party_id(consumer)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(consumer_wallet.escrow_balance, Decimal::ZERO);

    let supplier_wallet = wallet_repo
        .find_by_party_id(supplier)
        .await
        .unwrap()
        .unwrap();
    let enhancer_wallet = wallet_repo
        .find_by_party_id(enhancer)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(supplier_wallet.balance, Decimal::from(7000));
    assert_eq!(enhancer_wallet.balance, Decimal::from(3000));
}

#[tokio::test]
async fn settlement_succeeds_with_zero_enhancer_share() {
    let (_party_repo, deal_repo, wallet_repo, deal_id, supplier, consumer, enhancer) =
        executing_deal_fixture().await;

    let mut distribution = distribution(deal_id);
    distribution.enhancer_share_amount = Decimal::ZERO;
    distribution.enhancer_share_percentage = Decimal::ZERO;
    distribution.supplier_share_amount = Decimal::from(9000);
    distribution.platform_fee_amount = Decimal::from(1000);
    deal_repo
        .set_value_distribution(&distribution)
        .await
        .unwrap();

    fund_consumer_escrow(&wallet_repo, consumer, Decimal::from(10000)).await;

    let saga = SettlementSaga::new(deal_repo.clone(), wallet_repo.clone());
    let result = saga.execute(deal_id, actor_user_id()).await.unwrap();

    // Fee and supplier release only.
    assert_eq!(result.transaction_ids.len(), 2);

    let consumer_wallet = wallet_repo
        .find_by_party_id(consumer)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(consumer_wallet.escrow_balance, Decimal::ZERO);

    let supplier_wallet = wallet_repo
        .find_by_party_id(supplier)
        .await
        .unwrap()
        .unwrap();
    let enhancer_wallet = wallet_repo
        .find_by_party_id(enhancer)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(supplier_wallet.balance, Decimal::from(9000));
    assert_eq!(enhancer_wallet.balance, Decimal::ZERO);
}

#[tokio::test]
async fn settlement_happy_path_records_all_transactions_and_zeroes_escrow() {
    let (_party_repo, deal_repo, wallet_repo, deal_id, supplier, consumer, enhancer) =
        executing_deal_fixture().await;

    fund_consumer_escrow(&wallet_repo, consumer, Decimal::from(10000)).await;

    let saga = SettlementSaga::new(deal_repo.clone(), wallet_repo.clone());
    let result = saga.execute(deal_id, actor_user_id()).await.unwrap();

    assert_eq!(result.transaction_ids.len(), 3);

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
    let enhancer_wallet = wallet_repo
        .find_by_party_id(enhancer)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(consumer_wallet.balance, Decimal::ZERO);
    assert_eq!(consumer_wallet.escrow_balance, Decimal::ZERO);
    assert_eq!(supplier_wallet.balance, Decimal::from(6000));
    assert_eq!(enhancer_wallet.balance, Decimal::from(3000));

    let fee_txn = wallet_repo
        .find_transactions(
            consumer,
            &domain::repositories::TransactionFilters {
                deal_id: Some(deal_id),
                transaction_type: Some("FEE".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(fee_txn.len(), 1);
    assert_eq!(fee_txn[0].amount, Decimal::from(1000));
    assert_eq!(
        fee_txn[0].status,
        domain::entities::TransactionStatus::Verified
    );

    let releases = wallet_repo
        .find_transactions(
            supplier,
            &domain::repositories::TransactionFilters {
                deal_id: Some(deal_id),
                transaction_type: Some("ESCROW_RELEASE".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].amount, Decimal::from(6000));
    assert_eq!(releases[0].to_party_id, Some(supplier));
}
