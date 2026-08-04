use domain::entities::{
    ApprovalDecision, Currency, PlatformWallet, Transaction, TransactionApproval,
    TransactionStatus, TransactionType,
};
use domain::errors::DomainError;
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

async fn create_wallet(pool: &PgPool, party_id: Uuid) -> PlatformWallet {
    let repo = PostgresWalletRepository::new(pool.clone());
    let wallet = PlatformWallet::new(Uuid::now_v7(), party_id);
    repo.create(&wallet).await.unwrap();
    wallet
}

async fn seed_escrow(pool: &PgPool, party_id: Uuid, amount: Decimal, deal_id: Uuid) {
    let repo = PostgresWalletRepository::new(pool.clone());
    let mut wallet = repo.find_by_party_id(party_id).await.unwrap().unwrap();
    wallet.deposit(amount).unwrap();
    let deposit = Transaction::simple(
        Uuid::now_v7(),
        deal_id,
        TransactionType::Deposit,
        party_id,
        amount,
        None,
    );
    repo.record_transaction(&wallet, &deposit).await.unwrap();

    let mut wallet = repo.find_by_party_id(party_id).await.unwrap().unwrap();
    wallet.hold_escrow(amount).unwrap();
    let hold = Transaction::simple(
        Uuid::now_v7(),
        deal_id,
        TransactionType::EscrowHold,
        party_id,
        amount,
        None,
    );
    repo.record_transaction(&wallet, &hold).await.unwrap();
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

#[sqlx::test(migrations = "../../migrations")]
async fn records_pending_transaction_without_changing_balances(pool: PgPool) {
    let supplier_owner = create_user(&pool).await;
    let consumer_owner = create_user(&pool).await;
    let enhancer_owner = create_user(&pool).await;
    let supplier = create_party(&pool, supplier_owner).await;
    let consumer = create_party(&pool, consumer_owner).await;
    let enhancer = create_party(&pool, enhancer_owner).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    create_wallet(&pool, consumer).await;
    seed_escrow(&pool, consumer, Decimal::from(300), deal_id).await;
    create_wallet(&pool, supplier).await;

    let repo = PostgresWalletRepository::new(pool);
    let txn = Transaction::new_pending(
        Uuid::now_v7(),
        deal_id,
        TransactionType::EscrowRelease,
        Some(consumer),
        Some(supplier),
        Decimal::from(100),
        3,
        vec![consumer, supplier, enhancer],
        Some("milestone release".to_string()),
        None,
        None,
    );
    repo.record_pending_transaction(&txn).await.unwrap();

    let stored = repo.find_transaction_by_id(txn.id).await.unwrap().unwrap();
    assert_eq!(stored.status, TransactionStatus::Pending);
    assert_eq!(stored.approvals_received, 0);

    let consumer_wallet = repo.find_by_party_id(consumer).await.unwrap().unwrap();
    assert_eq!(consumer_wallet.escrow_balance, Decimal::from(300));
}

#[sqlx::test(migrations = "../../migrations")]
async fn records_approvals_and_finalises_escrow_release(pool: PgPool) {
    let supplier_owner = create_user(&pool).await;
    let consumer_owner = create_user(&pool).await;
    let enhancer_owner = create_user(&pool).await;
    let supplier = create_party(&pool, supplier_owner).await;
    let consumer = create_party(&pool, consumer_owner).await;
    let enhancer = create_party(&pool, enhancer_owner).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    create_wallet(&pool, consumer).await;
    seed_escrow(&pool, consumer, Decimal::from(300), deal_id).await;
    create_wallet(&pool, supplier).await;

    let repo = PostgresWalletRepository::new(pool.clone());
    let txn = Transaction::new_pending(
        Uuid::now_v7(),
        deal_id,
        TransactionType::EscrowRelease,
        Some(consumer),
        Some(supplier),
        Decimal::from(100),
        3,
        vec![consumer, supplier, enhancer],
        None,
        None,
        None,
    );
    repo.record_pending_transaction(&txn).await.unwrap();

    let parties = [consumer, supplier, enhancer];
    for (i, party_id) in parties.iter().enumerate() {
        let approver = create_user(&pool).await;
        let stored = repo.find_transaction_by_id(txn.id).await.unwrap().unwrap();
        let approval = TransactionApproval::new(
            Uuid::now_v7(),
            txn.id,
            *party_id,
            approver,
            ApprovalDecision::Approved,
            None,
        );

        let mutations: &[(Uuid, PlatformWallet)] = if i == parties.len() - 1 {
            let mut source = repo.find_by_party_id(consumer).await.unwrap().unwrap();
            let mut recipient = repo.find_by_party_id(supplier).await.unwrap().unwrap();
            source.debit_escrow(Decimal::from(100)).unwrap();
            recipient.credit_balance(Decimal::from(100)).unwrap();
            &[(consumer, source), (supplier, recipient)]
        } else {
            &[]
        };

        repo.record_approval_and_finalise(&stored, &approval, mutations)
            .await
            .unwrap();
    }

    let final_txn = repo.find_transaction_by_id(txn.id).await.unwrap().unwrap();
    assert_eq!(final_txn.status, TransactionStatus::Verified);
    assert_eq!(final_txn.approvals_received, 3);
    assert!(final_txn.executed_at.is_some());

    let consumer_wallet = repo.find_by_party_id(consumer).await.unwrap().unwrap();
    let supplier_wallet = repo.find_by_party_id(supplier).await.unwrap().unwrap();
    assert_eq!(consumer_wallet.escrow_balance, Decimal::from(200));
    assert_eq!(supplier_wallet.balance, Decimal::from(100));
}

#[sqlx::test(migrations = "../../migrations")]
async fn rejection_leaves_balances_unchanged(pool: PgPool) {
    let supplier_owner = create_user(&pool).await;
    let consumer_owner = create_user(&pool).await;
    let enhancer_owner = create_user(&pool).await;
    let supplier = create_party(&pool, supplier_owner).await;
    let consumer = create_party(&pool, consumer_owner).await;
    let enhancer = create_party(&pool, enhancer_owner).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    create_wallet(&pool, consumer).await;
    seed_escrow(&pool, consumer, Decimal::from(300), deal_id).await;
    create_wallet(&pool, supplier).await;

    let repo = PostgresWalletRepository::new(pool);
    let txn = Transaction::new_pending(
        Uuid::now_v7(),
        deal_id,
        TransactionType::EscrowRelease,
        Some(consumer),
        Some(supplier),
        Decimal::from(100),
        3,
        vec![consumer, supplier, enhancer],
        None,
        None,
        None,
    );
    repo.record_pending_transaction(&txn).await.unwrap();

    let approval = TransactionApproval::new(
        Uuid::now_v7(),
        txn.id,
        consumer,
        consumer_owner,
        ApprovalDecision::Rejected,
        Some("dispute".to_string()),
    );
    repo.record_approval_and_finalise(&txn, &approval, &[])
        .await
        .unwrap();

    let stored = repo.find_transaction_by_id(txn.id).await.unwrap().unwrap();
    assert_eq!(stored.status, TransactionStatus::Rejected);

    let consumer_wallet = repo.find_by_party_id(consumer).await.unwrap().unwrap();
    assert_eq!(consumer_wallet.escrow_balance, Decimal::from(300));
}

#[sqlx::test(migrations = "../../migrations")]
async fn finds_pending_transactions_for_party(pool: PgPool) {
    let supplier_owner = create_user(&pool).await;
    let consumer_owner = create_user(&pool).await;
    let enhancer_owner = create_user(&pool).await;
    let supplier = create_party(&pool, supplier_owner).await;
    let consumer = create_party(&pool, consumer_owner).await;
    let enhancer = create_party(&pool, enhancer_owner).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    create_wallet(&pool, consumer).await;
    create_wallet(&pool, supplier).await;
    create_wallet(&pool, enhancer).await;

    let repo = PostgresWalletRepository::new(pool);
    let txn = Transaction::new_pending(
        Uuid::now_v7(),
        deal_id,
        TransactionType::EscrowRelease,
        Some(consumer),
        Some(supplier),
        Decimal::from(100),
        3,
        vec![consumer, supplier, enhancer],
        None,
        None,
        None,
    );
    repo.record_pending_transaction(&txn).await.unwrap();

    for party_id in [consumer, supplier, enhancer] {
        let pending = repo
            .find_pending_transactions_for_party(party_id, 10, 0)
            .await
            .unwrap();
        assert_eq!(pending.len(), 1, "party {party_id} should see pending txn");
    }

    let approval = TransactionApproval::new(
        Uuid::now_v7(),
        txn.id,
        consumer,
        consumer_owner,
        ApprovalDecision::Approved,
        None,
    );
    repo.record_approval_and_finalise(&txn, &approval, &[])
        .await
        .unwrap();

    let consumer_pending = repo
        .find_pending_transactions_for_party(consumer, 10, 0)
        .await
        .unwrap();
    assert!(consumer_pending.is_empty());

    let supplier_pending = repo
        .find_pending_transactions_for_party(supplier, 10, 0)
        .await
        .unwrap();
    assert_eq!(supplier_pending.len(), 1);

    let count = repo
        .count_pending_transactions_for_party(enhancer)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn record_multi_party_transaction_updates_all_wallets(pool: PgPool) {
    let supplier_owner = create_user(&pool).await;
    let consumer_owner = create_user(&pool).await;
    let enhancer_owner = create_user(&pool).await;
    let supplier = create_party(&pool, supplier_owner).await;
    let consumer = create_party(&pool, consumer_owner).await;
    let enhancer = create_party(&pool, enhancer_owner).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    create_wallet(&pool, consumer).await;
    seed_escrow(&pool, consumer, Decimal::from(10000), deal_id).await;
    create_wallet(&pool, supplier).await;
    create_wallet(&pool, enhancer).await;

    let repo = PostgresWalletRepository::new(pool);

    let mut consumer_wallet = repo.find_by_party_id(consumer).await.unwrap().unwrap();
    let mut supplier_wallet = repo.find_by_party_id(supplier).await.unwrap().unwrap();

    consumer_wallet
        .deduct_fee_from_escrow(Decimal::from(1000))
        .unwrap();
    consumer_wallet.debit_escrow(Decimal::from(6000)).unwrap();
    supplier_wallet.credit_balance(Decimal::from(6000)).unwrap();

    let release = Transaction::new(
        Uuid::now_v7(),
        deal_id,
        TransactionType::EscrowRelease,
        Some(consumer),
        Some(supplier),
        Decimal::from(6000),
        Some("supplier share".to_string()),
        TransactionStatus::Verified,
        None,
        None,
    );

    repo.record_multi_party_transaction(
        &[consumer_wallet.clone(), supplier_wallet.clone()],
        &release,
    )
    .await
    .unwrap();

    let stored_consumer = repo.find_by_party_id(consumer).await.unwrap().unwrap();
    assert_eq!(stored_consumer.escrow_balance, Decimal::from(3000));

    let stored_supplier = repo.find_by_party_id(supplier).await.unwrap().unwrap();
    assert_eq!(stored_supplier.balance, Decimal::from(6000));

    let txns = repo
        .find_transactions(
            consumer,
            &TransactionFilters {
                deal_id: Some(deal_id),
                transaction_type: Some("ESCROW_RELEASE".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(txns.len(), 1);
    assert_eq!(txns[0].amount, Decimal::from(6000));
    assert_eq!(txns[0].status, TransactionStatus::Verified);
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_by_party_id_returns_none_for_party_without_wallet(pool: PgPool) {
    let owner = create_user(&pool).await;
    let party = create_party(&pool, owner).await;
    let repo = PostgresWalletRepository::new(pool);

    assert!(repo.find_by_party_id(party).await.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_persists_wallet_changes(pool: PgPool) {
    let owner = create_user(&pool).await;
    let party = create_party(&pool, owner).await;
    let repo = PostgresWalletRepository::new(pool);

    let wallet = PlatformWallet::new(Uuid::now_v7(), party);
    repo.create(&wallet).await.unwrap();

    let mut wallet = repo.find_by_party_id(party).await.unwrap().unwrap();
    wallet.deposit(Decimal::from(250)).unwrap();
    wallet.mark_inactive();
    repo.update(&wallet).await.unwrap();

    let stored = repo.find_by_party_id(party).await.unwrap().unwrap();
    assert_eq!(stored.balance, Decimal::from(250));
    assert_eq!(stored.total_deposited, Decimal::from(250));
    assert!(!stored.is_active);
}

#[sqlx::test(migrations = "../../migrations")]
async fn records_withdrawal_and_filters_by_status_and_type(pool: PgPool) {
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
    let deposit = Transaction::simple(
        Uuid::now_v7(),
        deal_id,
        TransactionType::Deposit,
        party,
        Decimal::from(500),
        None,
    );
    repo.record_transaction(&wallet, &deposit).await.unwrap();

    let mut wallet = repo.find_by_party_id(party).await.unwrap().unwrap();
    wallet.withdraw(Decimal::from(150)).unwrap();
    let withdrawal = Transaction::simple(
        Uuid::now_v7(),
        deal_id,
        TransactionType::Withdrawal,
        party,
        Decimal::from(150),
        None,
    );
    repo.record_transaction(&wallet, &withdrawal).await.unwrap();

    let verified = repo
        .find_transactions(
            party,
            &TransactionFilters {
                status: Some("VERIFIED".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(verified.len(), 2);

    let count = repo
        .count_transactions(
            party,
            &TransactionFilters {
                status: Some("VERIFIED".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(count, 2);

    let deposits = repo
        .find_transactions(
            party,
            &TransactionFilters {
                transaction_type: Some("DEPOSIT".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(deposits.len(), 1);
    assert_eq!(deposits[0].amount, Decimal::from(500));

    let withdrawals = repo
        .find_transactions(
            party,
            &TransactionFilters {
                transaction_type: Some("WITHDRAWAL".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(withdrawals.len(), 1);
    assert_eq!(withdrawals[0].amount, Decimal::from(150));
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_transactions_pagination(pool: PgPool) {
    let owner = create_user(&pool).await;
    let party = create_party(&pool, owner).await;
    let supplier = create_party(&pool, create_user(&pool).await).await;
    let consumer = create_party(&pool, create_user(&pool).await).await;
    let enhancer = create_party(&pool, create_user(&pool).await).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    let repo = PostgresWalletRepository::new(pool);
    let wallet = PlatformWallet::new(Uuid::now_v7(), party);
    repo.create(&wallet).await.unwrap();

    for amount in [Decimal::from(100), Decimal::from(200)] {
        let mut wallet = repo.find_by_party_id(party).await.unwrap().unwrap();
        wallet.deposit(amount).unwrap();
        let txn = Transaction::simple(
            Uuid::now_v7(),
            deal_id,
            TransactionType::Deposit,
            party,
            amount,
            None,
        );
        repo.record_transaction(&wallet, &txn).await.unwrap();
    }

    let first = repo
        .find_transactions(
            party,
            &TransactionFilters {
                limit: 1,
                offset: 0,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(first.len(), 1);

    let second = repo
        .find_transactions(
            party,
            &TransactionFilters {
                limit: 1,
                offset: 1,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(second.len(), 1);

    let beyond = repo
        .find_transactions(
            party,
            &TransactionFilters {
                limit: 10,
                offset: 2,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(beyond.is_empty());

    let count = repo
        .count_transactions(party, &TransactionFilters::default())
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn compute_deal_wallet_with_all_transaction_types(pool: PgPool) {
    let consumer_owner = create_user(&pool).await;
    let consumer = create_party(&pool, consumer_owner).await;
    let supplier = create_party(&pool, create_user(&pool).await).await;
    let enhancer = create_party(&pool, create_user(&pool).await).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    let repo = PostgresWalletRepository::new(pool);
    let wallet = PlatformWallet::new(Uuid::now_v7(), consumer);
    repo.create(&wallet).await.unwrap();

    let mut wallet = repo.find_by_party_id(consumer).await.unwrap().unwrap();
    wallet.deposit(Decimal::from(1000)).unwrap();
    let deposit = Transaction::simple(
        Uuid::now_v7(),
        deal_id,
        TransactionType::Deposit,
        consumer,
        Decimal::from(500),
        None,
    );
    repo.record_transaction(&wallet, &deposit).await.unwrap();

    let mut wallet = repo.find_by_party_id(consumer).await.unwrap().unwrap();
    wallet.hold_escrow(Decimal::from(300)).unwrap();
    let hold = Transaction::simple(
        Uuid::now_v7(),
        deal_id,
        TransactionType::EscrowHold,
        consumer,
        Decimal::from(300),
        None,
    );
    repo.record_transaction(&wallet, &hold).await.unwrap();

    let mut wallet = repo.find_by_party_id(consumer).await.unwrap().unwrap();
    wallet.withdraw(Decimal::from(100)).unwrap();
    let withdrawal = Transaction::simple(
        Uuid::now_v7(),
        deal_id,
        TransactionType::Withdrawal,
        consumer,
        Decimal::from(100),
        None,
    );
    repo.record_transaction(&wallet, &withdrawal).await.unwrap();

    let mut wallet = repo.find_by_party_id(consumer).await.unwrap().unwrap();
    wallet.deduct_fee_from_balance(Decimal::from(50)).unwrap();
    let fee = Transaction::new(
        Uuid::now_v7(),
        deal_id,
        TransactionType::Fee,
        Some(consumer),
        None,
        Decimal::from(50),
        None,
        TransactionStatus::Verified,
        None,
        None,
    );
    repo.record_transaction(&wallet, &fee).await.unwrap();

    let mut wallet = repo.find_by_party_id(consumer).await.unwrap().unwrap();
    wallet.credit_balance(Decimal::from(200)).unwrap();
    let release = Transaction::new(
        Uuid::now_v7(),
        deal_id,
        TransactionType::EscrowRelease,
        Some(supplier),
        Some(consumer),
        Decimal::from(200),
        None,
        TransactionStatus::Verified,
        None,
        None,
    );
    repo.record_transaction(&wallet, &release).await.unwrap();

    let mut wallet = repo.find_by_party_id(consumer).await.unwrap().unwrap();
    wallet.credit_balance(Decimal::from(25)).unwrap();
    let adjustment_to = Transaction::new(
        Uuid::now_v7(),
        deal_id,
        TransactionType::Adjustment,
        None,
        Some(consumer),
        Decimal::from(25),
        None,
        TransactionStatus::Verified,
        None,
        None,
    );
    repo.record_transaction(&wallet, &adjustment_to)
        .await
        .unwrap();

    let mut wallet = repo.find_by_party_id(consumer).await.unwrap().unwrap();
    wallet.deduct_fee_from_balance(Decimal::from(15)).unwrap();
    let adjustment_from = Transaction::new(
        Uuid::now_v7(),
        deal_id,
        TransactionType::Adjustment,
        Some(consumer),
        None,
        Decimal::from(15),
        None,
        TransactionStatus::Verified,
        None,
        None,
    );
    repo.record_transaction(&wallet, &adjustment_from)
        .await
        .unwrap();

    let deal_wallet = repo
        .compute_deal_wallet(consumer, deal_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(deal_wallet.deposited, Decimal::from(500));
    assert_eq!(deal_wallet.withdrawn, Decimal::from(100));
    assert_eq!(deal_wallet.contributed, Decimal::from(715));
    assert_eq!(deal_wallet.held_in_escrow, Decimal::from(100));
    assert_eq!(deal_wallet.released, Decimal::from(225));
    assert_eq!(deal_wallet.fees_paid, Decimal::from(50));
    assert_eq!(deal_wallet.net_position, Decimal::from(-440));
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_approvals_for_transaction_returns_recorded_approvals(pool: PgPool) {
    let consumer_owner = create_user(&pool).await;
    let supplier_owner = create_user(&pool).await;
    let enhancer_owner = create_user(&pool).await;
    let consumer = create_party(&pool, consumer_owner).await;
    let supplier = create_party(&pool, supplier_owner).await;
    let enhancer = create_party(&pool, enhancer_owner).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    create_wallet(&pool, consumer).await;
    create_wallet(&pool, supplier).await;
    create_wallet(&pool, enhancer).await;

    let repo = PostgresWalletRepository::new(pool);
    let txn = Transaction::new_pending(
        Uuid::now_v7(),
        deal_id,
        TransactionType::EscrowRelease,
        Some(consumer),
        Some(supplier),
        Decimal::from(100),
        2,
        vec![consumer, supplier],
        None,
        None,
        None,
    );
    repo.record_pending_transaction(&txn).await.unwrap();

    let empty = repo.find_approvals_for_transaction(txn.id).await.unwrap();
    assert!(empty.is_empty());

    let approval = TransactionApproval::new(
        Uuid::now_v7(),
        txn.id,
        consumer,
        consumer_owner,
        ApprovalDecision::Approved,
        Some("looks good".to_string()),
    );
    repo.record_approval_and_finalise(&txn, &approval, &[])
        .await
        .unwrap();

    let approvals = repo.find_approvals_for_transaction(txn.id).await.unwrap();
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].decision, ApprovalDecision::Approved);
    assert_eq!(approvals[0].comment.as_deref(), Some("looks good"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn intermediate_approval_keeps_transaction_pending(pool: PgPool) {
    let consumer_owner = create_user(&pool).await;
    let supplier_owner = create_user(&pool).await;
    let enhancer_owner = create_user(&pool).await;
    let consumer = create_party(&pool, consumer_owner).await;
    let supplier = create_party(&pool, supplier_owner).await;
    let enhancer = create_party(&pool, enhancer_owner).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    create_wallet(&pool, consumer).await;
    create_wallet(&pool, supplier).await;
    create_wallet(&pool, enhancer).await;

    let repo = PostgresWalletRepository::new(pool);
    let txn = Transaction::new_pending(
        Uuid::now_v7(),
        deal_id,
        TransactionType::EscrowRelease,
        Some(consumer),
        Some(supplier),
        Decimal::from(100),
        3,
        vec![consumer, supplier, enhancer],
        None,
        None,
        None,
    );
    repo.record_pending_transaction(&txn).await.unwrap();

    let first = TransactionApproval::new(
        Uuid::now_v7(),
        txn.id,
        consumer,
        consumer_owner,
        ApprovalDecision::Approved,
        None,
    );
    repo.record_approval_and_finalise(&txn, &first, &[])
        .await
        .unwrap();

    let stored = repo.find_transaction_by_id(txn.id).await.unwrap().unwrap();
    assert_eq!(stored.status, TransactionStatus::Pending);
    assert_eq!(stored.approvals_received, 1);

    let second = TransactionApproval::new(
        Uuid::now_v7(),
        txn.id,
        supplier,
        supplier_owner,
        ApprovalDecision::Approved,
        None,
    );
    repo.record_approval_and_finalise(&stored, &second, &[])
        .await
        .unwrap();

    let stored = repo.find_transaction_by_id(txn.id).await.unwrap().unwrap();
    assert_eq!(stored.status, TransactionStatus::Pending);
    assert_eq!(stored.approvals_received, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_pending_transactions_pagination(pool: PgPool) {
    let consumer_owner = create_user(&pool).await;
    let supplier_owner = create_user(&pool).await;
    let enhancer_owner = create_user(&pool).await;
    let consumer = create_party(&pool, consumer_owner).await;
    let supplier = create_party(&pool, supplier_owner).await;
    let enhancer = create_party(&pool, enhancer_owner).await;
    let deal_id = create_deal(&pool, supplier, consumer, enhancer).await;

    create_wallet(&pool, consumer).await;
    create_wallet(&pool, supplier).await;
    create_wallet(&pool, enhancer).await;

    let repo = PostgresWalletRepository::new(pool);
    for i in 0..2 {
        let txn = Transaction::new_pending(
            Uuid::now_v7(),
            deal_id,
            TransactionType::EscrowRelease,
            Some(consumer),
            Some(supplier),
            Decimal::from(10 + i),
            3,
            vec![consumer, supplier, enhancer],
            None,
            None,
            None,
        );
        repo.record_pending_transaction(&txn).await.unwrap();
    }

    let first = repo
        .find_pending_transactions_for_party(consumer, 1, 0)
        .await
        .unwrap();
    assert_eq!(first.len(), 1);

    let second = repo
        .find_pending_transactions_for_party(consumer, 1, 1)
        .await
        .unwrap();
    assert_eq!(second.len(), 1);

    let beyond = repo
        .find_pending_transactions_for_party(consumer, 10, 2)
        .await
        .unwrap();
    assert!(beyond.is_empty());

    let count = repo
        .count_pending_transactions_for_party(consumer)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_duplicate_wallet_id_returns_repository_error(pool: PgPool) {
    let owner = create_user(&pool).await;
    let party = create_party(&pool, owner).await;
    let other_owner = create_user(&pool).await;
    let other_party = create_party(&pool, other_owner).await;

    let repo = PostgresWalletRepository::new(pool);
    let id = Uuid::now_v7();

    let wallet = PlatformWallet::new(id, party);
    repo.create(&wallet).await.unwrap();

    let duplicate = PlatformWallet::new(id, other_party);
    let result = repo.create(&duplicate).await;
    assert!(
        matches!(result, Err(DomainError::RepositoryError(_))),
        "expected repository error for duplicate wallet id, got {result:?}"
    );
}
