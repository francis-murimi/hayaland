use domain::entities::{Currency, PlatformWallet, TransactionType};
use domain::repositories::{TransactionFilters, WalletRepository};
use infrastructure::repositories::PostgresWalletRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

async fn create_user(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO users (id, email, username, password_hash, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, true, now(), now())
        "#,
        id,
        format!("user-{id}@example.com"),
        format!("user-{id}"),
        "hash"
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn create_party(pool: &PgPool, owner_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO parties (
            id, party_type, display_name, email, verification_status,
            is_active, created_at, updated_at
        )
        VALUES ($1, 'ORGANIZATION', $2, $3, 'UNVERIFIED', true, now(), now())
        "#,
        id,
        format!("Party {id}"),
        format!("party-{id}@example.com"),
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!(
        r#"
        INSERT INTO user_party_memberships (id, user_id, party_id, member_role, is_active, created_at)
        VALUES ($1, $2, $3, 'OWNER', true, now())
        "#,
        Uuid::now_v7(),
        owner_id,
        id
    )
    .execute(pool)
    .await
    .unwrap();

    id
}

async fn create_category(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    let code = format!("CAT-{id}");
    sqlx::query!(
        r#"
        INSERT INTO categories (id, category_name, category_code, category_type, created_at, updated_at)
        VALUES ($1, $2, $3, 'RESOURCE_TYPE', now(), now())
        "#,
        id,
        format!("Category {id}"),
        code
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn create_deal(pool: &PgPool, supplier: Uuid, consumer: Uuid, enhancer: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    let category_id = create_category(pool).await;
    sqlx::query!(
        r#"
        INSERT INTO deals (
            id, deal_reference, deal_title, domain_category_id, initiator_party_id,
            initiator_role, deal_status, created_at, updated_at
        )
        VALUES ($1, $2, 'Test Deal', $3, $4, 'SUPPLIER', 'DRAFT', now(), now())
        "#,
        id,
        format!("DEAL-{id}"),
        category_id,
        supplier
    )
    .execute(pool)
    .await
    .unwrap();

    for (party_id, role) in [
        (supplier, "SUPPLIER"),
        (consumer, "CONSUMER"),
        (enhancer, "ENHANCER"),
    ] {
        sqlx::query!(
            r#"
            INSERT INTO deal_participations (
                id, deal_id, party_id, role, participation_status,
                is_initiator, created_at
            )
            VALUES ($1, $2, $3, $4, 'ACCEPTED', $5, now())
            "#,
            Uuid::now_v7(),
            id,
            party_id,
            role,
            role == "SUPPLIER"
        )
        .execute(pool)
        .await
        .unwrap();
    }

    id
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_wallet(pool: PgPool) {
    let owner = create_user(&pool).await;
    let party = create_party(&pool, owner).await;
    let repo = PostgresWalletRepository::new(pool);

    let wallet = PlatformWallet::new(Uuid::now_v7(), party);
    repo.create(&wallet).await.unwrap();

    let found = repo.find_by_party_id(party).await.unwrap().unwrap();
    assert_eq!(found.party_id, party);
    assert_eq!(found.balance, Decimal::ZERO);
    assert_eq!(found.currency, Currency::Points);
}

#[sqlx::test(migrations = "../../migrations")]
async fn records_deposit_and_updates_balance(pool: PgPool) {
    let owner = create_user(&pool).await;
    let party = create_party(&pool, owner).await;
    let supplier = create_party(&pool, create_user(&pool).await).await;
    let consumer = create_party(&pool, create_user(&pool).await).await;
    let enhancer = create_party(&pool, create_user(&pool).await).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    let repo = PostgresWalletRepository::new(pool);
    let wallet = PlatformWallet::new(Uuid::now_v7(), party);
    repo.create(&wallet).await.unwrap();

    let mut wallet = repo.find_by_party_id(party).await.unwrap().unwrap();
    wallet.deposit(Decimal::from(1000)).unwrap();

    let txn = domain::entities::Transaction::simple(
        Uuid::now_v7(),
        deal_id,
        TransactionType::Deposit,
        party,
        Decimal::from(1000),
        Some("test deposit".to_string()),
    );
    repo.record_transaction(&wallet, &txn).await.unwrap();

    let found = repo.find_by_party_id(party).await.unwrap().unwrap();
    assert_eq!(found.balance, Decimal::from(1000));
    assert_eq!(found.total_deposited, Decimal::from(1000));

    let txns = repo
        .find_transactions(party, &TransactionFilters::default())
        .await
        .unwrap();
    assert_eq!(txns.len(), 1);
    assert_eq!(txns[0].transaction_type, TransactionType::Deposit);
    assert_eq!(txns[0].deal_id, deal_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn filters_transactions_by_deal(pool: PgPool) {
    let owner = create_user(&pool).await;
    let party = create_party(&pool, owner).await;
    let supplier = create_party(&pool, create_user(&pool).await).await;
    let consumer = create_party(&pool, create_user(&pool).await).await;
    let enhancer = create_party(&pool, create_user(&pool).await).await;
    let deal_a = create_deal(&pool, supplier, consumer, enhancer).await;
    let deal_b = create_deal(&pool, supplier, consumer, enhancer).await;

    let repo = PostgresWalletRepository::new(pool);
    let wallet = PlatformWallet::new(Uuid::now_v7(), party);
    repo.create(&wallet).await.unwrap();

    for deal_id in [deal_a, deal_b] {
        let mut wallet = repo.find_by_party_id(party).await.unwrap().unwrap();
        wallet.deposit(Decimal::from(100)).unwrap();
        let txn = domain::entities::Transaction::simple(
            Uuid::now_v7(),
            deal_id,
            TransactionType::Deposit,
            party,
            Decimal::from(100),
            None,
        );
        repo.record_transaction(&wallet, &txn).await.unwrap();
    }

    let filtered = repo
        .find_transactions(
            party,
            &TransactionFilters {
                deal_id: Some(deal_a),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].deal_id, deal_a);
}

#[sqlx::test(migrations = "../../migrations")]
async fn computes_deal_wallet(pool: PgPool) {
    let owner = create_user(&pool).await;
    let party = create_party(&pool, owner).await;
    let supplier = create_party(&pool, create_user(&pool).await).await;
    let consumer = create_party(&pool, create_user(&pool).await).await;
    let enhancer = create_party(&pool, create_user(&pool).await).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    let repo = PostgresWalletRepository::new(pool);
    let wallet = PlatformWallet::new(Uuid::now_v7(), party);
    repo.create(&wallet).await.unwrap();

    let mut wallet = repo.find_by_party_id(party).await.unwrap().unwrap();
    wallet.deposit(Decimal::from(500)).unwrap();
    let deposit = domain::entities::Transaction::simple(
        Uuid::now_v7(),
        deal_id,
        TransactionType::Deposit,
        party,
        Decimal::from(500),
        None,
    );
    repo.record_transaction(&wallet, &deposit).await.unwrap();

    let mut wallet = repo.find_by_party_id(party).await.unwrap().unwrap();
    wallet.hold_escrow(Decimal::from(300)).unwrap();
    let hold = domain::entities::Transaction::simple(
        Uuid::now_v7(),
        deal_id,
        TransactionType::EscrowHold,
        party,
        Decimal::from(300),
        None,
    );
    repo.record_transaction(&wallet, &hold).await.unwrap();

    let deal_wallet = repo
        .compute_deal_wallet(party, deal_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(deal_wallet.deposited, Decimal::from(500));
    assert_eq!(deal_wallet.contributed, Decimal::from(800));
    assert_eq!(deal_wallet.held_in_escrow, Decimal::from(300));
}

#[sqlx::test(migrations = "../../migrations")]
async fn returns_none_for_deal_wallet_without_activity(pool: PgPool) {
    let owner = create_user(&pool).await;
    let party = create_party(&pool, owner).await;
    let supplier = create_party(&pool, create_user(&pool).await).await;
    let consumer = create_party(&pool, create_user(&pool).await).await;
    let enhancer = create_party(&pool, create_user(&pool).await).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    let repo = PostgresWalletRepository::new(pool);
    let wallet = PlatformWallet::new(Uuid::now_v7(), party);
    repo.create(&wallet).await.unwrap();

    let deal_wallet = repo.compute_deal_wallet(party, deal_id).await.unwrap();
    assert!(deal_wallet.is_none());
}
