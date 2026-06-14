use domain::entities::{
    Agreement, AgreementStatus, Deal, DealParticipation, DealRole, DealTitle, DisplayName, Email,
    Party, PartyType, Signature, SignatureType, User,
};
use domain::repositories::{
    AgreementRepository, DealAggregate, DealRepository, PartyRepository, UserRepository,
};
use infrastructure::repositories::{
    PostgresAgreementRepository, PostgresDealRepository, PostgresPartyRepository,
    PostgresUserRepository,
};
use sqlx::PgPool;
use uuid::Uuid;

fn agriculture_category_id() -> Uuid {
    Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap()
}

fn sample_party(email: &str, display_name: &str) -> Party {
    Party::new(
        Uuid::now_v7(),
        PartyType::Organization,
        DisplayName::new(display_name).unwrap(),
        Email::new(email).unwrap(),
    )
}

async fn three_party_fixture(party_repo: &PostgresPartyRepository) -> (Uuid, Uuid, Uuid) {
    let supplier = sample_party("supplier@example.com", "Supplier");
    let consumer = sample_party("consumer@example.com", "Consumer");
    let enhancer = sample_party("enhancer@example.com", "Enhancer");

    party_repo.create(&supplier).await.unwrap();
    party_repo.create(&consumer).await.unwrap();
    party_repo.create(&enhancer).await.unwrap();

    (supplier.id, consumer.id, enhancer.id)
}

async fn create_user(user_repo: &PostgresUserRepository) -> Uuid {
    let user = User::new(
        Uuid::now_v7(),
        Email::new("signer@example.com").unwrap(),
        domain::entities::Username::new("signer").unwrap(),
        domain::entities::PasswordHash::new("hashed:password".to_string()).unwrap(),
    );
    let id = user.id;
    user_repo.create(&user).await.unwrap();
    id
}

async fn sample_deal(
    deal_repo: &PostgresDealRepository,
    supplier: Uuid,
    consumer: Uuid,
    enhancer: Uuid,
) -> Uuid {
    let deal_id = Uuid::now_v7();
    let deal = Deal::new(
        deal_id,
        format!(
            "DL-2026-{}",
            Uuid::now_v7().to_string().split('-').next().unwrap()
        ),
        DealTitle::new("Agreement Test Deal").unwrap(),
        agriculture_category_id(),
        supplier,
        DealRole::Supplier,
    );

    let participations = vec![
        DealParticipation::new(Uuid::now_v7(), deal_id, supplier, DealRole::Supplier, true),
        DealParticipation::new(Uuid::now_v7(), deal_id, consumer, DealRole::Consumer, false),
        DealParticipation::new(Uuid::now_v7(), deal_id, enhancer, DealRole::Enhancer, false),
    ];

    deal_repo
        .create(&DealAggregate {
            deal,
            participations,
        })
        .await
        .unwrap();

    deal_id
}

fn sample_agreement(deal_id: Uuid) -> Agreement {
    Agreement::new(
        Uuid::now_v7(),
        deal_id,
        "# Agreement".to_string(),
        Some("CA".to_string()),
        Some("Arbitration".to_string()),
        None,
        1,
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_agreement(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool.clone());
    let agreement_repo = PostgresAgreementRepository::new(pool);

    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;
    let deal_id = sample_deal(&deal_repo, supplier, consumer, enhancer).await;

    let agreement = sample_agreement(deal_id);
    agreement_repo.create(&agreement).await.unwrap();

    let found_by_deal = agreement_repo
        .find_by_deal_id(deal_id)
        .await
        .unwrap()
        .expect("agreement should exist");
    assert_eq!(found_by_deal.id, agreement.id);
    assert_eq!(found_by_deal.deal_id, deal_id);
    assert_eq!(
        found_by_deal.agreement_status,
        AgreementStatus::PendingSignatures
    );

    let found_by_id = agreement_repo
        .find_by_id(agreement.id)
        .await
        .unwrap()
        .expect("agreement should exist");
    assert_eq!(found_by_id.id, agreement.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn updates_agreement(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool.clone());
    let agreement_repo = PostgresAgreementRepository::new(pool);

    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;
    let deal_id = sample_deal(&deal_repo, supplier, consumer, enhancer).await;

    let mut agreement = sample_agreement(deal_id);
    agreement_repo.create(&agreement).await.unwrap();

    agreement.mark_signed();
    agreement.governing_law = Some("Delaware".to_string());
    agreement_repo.update(&agreement).await.unwrap();

    let found = agreement_repo
        .find_by_id(agreement.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.agreement_status, AgreementStatus::Signed);
    assert_eq!(found.governing_law, Some("Delaware".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_queries_signatures(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool.clone());
    let agreement_repo = PostgresAgreementRepository::new(pool.clone());

    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;
    let deal_id = sample_deal(&deal_repo, supplier, consumer, enhancer).await;

    let agreement = sample_agreement(deal_id);
    agreement_repo.create(&agreement).await.unwrap();

    let user_repo = PostgresUserRepository::new(pool.clone());
    let user_id = create_user(&user_repo).await;
    let signature = Signature::new(
        Uuid::now_v7(),
        agreement.id,
        supplier,
        user_id,
        SignatureType::DigitalAttestation,
        "sha256:abc".to_string(),
        Some("127.0.0.1".to_string()),
        agreement.version,
    );

    agreement_repo.create_signature(&signature).await.unwrap();

    let signatures = agreement_repo
        .find_signatures_by_agreement(agreement.id)
        .await
        .unwrap();
    assert_eq!(signatures.len(), 1);
    assert_eq!(signatures[0].party_id, supplier);
    assert_eq!(signatures[0].version, agreement.version);

    assert!(agreement_repo
        .has_party_signed(agreement.id, supplier)
        .await
        .unwrap());
    assert!(!agreement_repo
        .has_party_signed(agreement.id, consumer)
        .await
        .unwrap());

    let count = agreement_repo.count_signatures(agreement.id).await.unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn signatures_are_tied_to_agreement_version(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool.clone());
    let agreement_repo = PostgresAgreementRepository::new(pool.clone());

    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;
    let deal_id = sample_deal(&deal_repo, supplier, consumer, enhancer).await;

    let mut agreement = sample_agreement(deal_id);
    agreement_repo.create(&agreement).await.unwrap();

    let user_repo = PostgresUserRepository::new(pool.clone());
    let user_id = create_user(&user_repo).await;
    let signature_v1 = Signature::new(
        Uuid::now_v7(),
        agreement.id,
        supplier,
        user_id,
        SignatureType::DigitalAttestation,
        "sha256:v1".to_string(),
        None,
        1,
    );
    agreement_repo
        .create_signature(&signature_v1)
        .await
        .unwrap();

    // Simulate renegotiation: bump version and require a fresh signature.
    agreement.version = 2;
    agreement_repo.update(&agreement).await.unwrap();

    let signature_v2 = Signature::new(
        Uuid::now_v7(),
        agreement.id,
        supplier,
        user_id,
        SignatureType::DigitalAttestation,
        "sha256:v2".to_string(),
        None,
        2,
    );
    agreement_repo
        .create_signature(&signature_v2)
        .await
        .unwrap();

    let count = agreement_repo.count_signatures(agreement.id).await.unwrap();
    assert_eq!(count, 1);

    let has_signed = agreement_repo
        .has_party_signed(agreement.id, supplier)
        .await
        .unwrap();
    assert!(has_signed);

    let signatures = agreement_repo
        .find_signatures_by_agreement(agreement.id)
        .await
        .unwrap();
    assert_eq!(signatures.len(), 2);
}
