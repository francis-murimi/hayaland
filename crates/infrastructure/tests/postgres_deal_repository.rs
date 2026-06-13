use domain::entities::{
    Deal, DealParticipation, DealRole, DealStatus, DealTitle, DisplayName, Email,
    ParticipationStatus, Party, PartyType, RoleProfile,
};
use domain::repositories::{DealAggregate, DealRepository, DealSearchCriteria, PartyRepository};
use infrastructure::repositories::{PostgresDealRepository, PostgresPartyRepository};
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::OffsetDateTime;
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

async fn create_party_with_role(
    repo: &PostgresPartyRepository,
    role: DealRole,
    email: &str,
    name: &str,
) -> Uuid {
    let party = sample_party(email, name);
    let id = party.id;
    repo.create(&party).await.unwrap();
    repo.add_role(id, role, RoleProfile::for_role(role))
        .await
        .unwrap();
    id
}

async fn three_party_fixture(party_repo: &PostgresPartyRepository) -> (Uuid, Uuid, Uuid) {
    let supplier = create_party_with_role(
        party_repo,
        DealRole::Supplier,
        "supplier@example.com",
        "Supplier",
    )
    .await;
    let consumer = create_party_with_role(
        party_repo,
        DealRole::Consumer,
        "consumer@example.com",
        "Consumer",
    )
    .await;
    let enhancer = create_party_with_role(
        party_repo,
        DealRole::Enhancer,
        "enhancer@example.com",
        "Enhancer",
    )
    .await;
    (supplier, consumer, enhancer)
}

fn sample_deal_aggregate(
    supplier: Uuid,
    consumer: Uuid,
    enhancer: Uuid,
    reference: &str,
) -> DealAggregate {
    let deal_id = Uuid::now_v7();
    let deal = Deal::new(
        deal_id,
        reference.to_string(),
        DealTitle::new("Sample Deal").unwrap(),
        agriculture_category_id(),
        supplier,
        DealRole::Supplier,
    );

    let participations = vec![
        DealParticipation::new(Uuid::now_v7(), deal_id, supplier, DealRole::Supplier, true),
        DealParticipation::new(Uuid::now_v7(), deal_id, consumer, DealRole::Consumer, false),
        DealParticipation::new(Uuid::now_v7(), deal_id, enhancer, DealRole::Enhancer, false),
    ];

    DealAggregate {
        deal,
        participations,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_aggregate(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool);
    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;

    let aggregate = sample_deal_aggregate(supplier, consumer, enhancer, "DL-2026-0001");
    let deal_id = aggregate.deal.id;

    deal_repo.create(&aggregate).await.unwrap();

    let found = deal_repo.find_aggregate_by_id(deal_id).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.deal.id, deal_id);
    assert_eq!(found.deal.deal_status, DealStatus::Draft);
    assert_eq!(found.participations.len(), 3);
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_by_id_returns_none_for_missing(pool: PgPool) {
    let deal_repo = PostgresDealRepository::new(pool);

    let found = deal_repo.find_by_id(Uuid::now_v7()).await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn updates_deal_status(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool);
    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;

    let aggregate = sample_deal_aggregate(supplier, consumer, enhancer, "DL-2026-0002");
    let deal_id = aggregate.deal.id;
    deal_repo.create(&aggregate).await.unwrap();

    let mut deal = aggregate.deal;
    deal.transition(DealStatus::Suggested).unwrap();
    deal_repo.update(&deal).await.unwrap();

    let found = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    assert_eq!(found.deal_status, DealStatus::Suggested);
    assert!(found.updated_at > found.created_at);
}

#[sqlx::test(migrations = "../../migrations")]
async fn updates_participation_status(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool);
    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;

    let aggregate = sample_deal_aggregate(supplier, consumer, enhancer, "DL-2026-0003");
    let deal_id = aggregate.deal.id;
    deal_repo.create(&aggregate).await.unwrap();

    let mut participation = deal_repo
        .find_participations_by_deal(deal_id)
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.party_id == consumer)
        .unwrap();

    participation.participation_status = ParticipationStatus::Accepted;
    participation.responded_at = Some(OffsetDateTime::now_utc());
    deal_repo
        .update_participation(&participation)
        .await
        .unwrap();

    let participations = deal_repo
        .find_participations_by_deal(deal_id)
        .await
        .unwrap();
    let consumer_part = participations
        .iter()
        .find(|p| p.party_id == consumer)
        .unwrap();
    assert_eq!(
        consumer_part.participation_status,
        ParticipationStatus::Accepted
    );
    assert!(consumer_part.responded_at.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn lists_deals_for_party_with_status_filter(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool);
    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;

    let aggregate = sample_deal_aggregate(supplier, consumer, enhancer, "DL-2026-0004");
    deal_repo.create(&aggregate).await.unwrap();

    let criteria = DealSearchCriteria {
        party_id: Some(supplier),
        status: Some(DealStatus::Draft),
        limit: 10,
        offset: 0,
        ..Default::default()
    };

    let result = deal_repo.list(&criteria).await.unwrap();
    assert_eq!(result.deals.len(), 1);
    assert_eq!(result.total, 1);

    let criteria_other_status = DealSearchCriteria {
        party_id: Some(supplier),
        status: Some(DealStatus::Suggested),
        limit: 10,
        offset: 0,
        ..Default::default()
    };
    let result = deal_repo.list(&criteria_other_status).await.unwrap();
    assert_eq!(result.deals.len(), 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn counts_active_deals_for_party_and_role(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool);
    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;

    let aggregate = sample_deal_aggregate(supplier, consumer, enhancer, "DL-2026-0005");
    deal_repo.create(&aggregate).await.unwrap();

    let count = deal_repo
        .count_active_deals_for_party(supplier)
        .await
        .unwrap();
    assert_eq!(count, 1);

    let count_role = deal_repo
        .count_active_deals_for_party_role(supplier, DealRole::Supplier)
        .await
        .unwrap();
    assert_eq!(count_role, 1);

    let count_other_role = deal_repo
        .count_active_deals_for_party_role(supplier, DealRole::Consumer)
        .await
        .unwrap();
    assert_eq!(count_other_role, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn records_history(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool.clone());
    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;

    let aggregate = sample_deal_aggregate(supplier, consumer, enhancer, "DL-2026-0006");
    let deal_id = aggregate.deal.id;
    deal_repo.create(&aggregate).await.unwrap();

    deal_repo
        .record_history(deal_id, "DEAL_CREATED", Some(supplier), None)
        .await
        .unwrap();

    let row = sqlx::query!(
        "SELECT event_type FROM deal_history WHERE deal_id = $1",
        deal_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.event_type, "DEAL_CREATED");
}

#[sqlx::test(migrations = "../../migrations")]
async fn checks_party_participation(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool);
    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;

    let aggregate = sample_deal_aggregate(supplier, consumer, enhancer, "DL-2026-0007");
    let deal_id = aggregate.deal.id;
    deal_repo.create(&aggregate).await.unwrap();

    assert!(deal_repo
        .is_party_participant(deal_id, supplier)
        .await
        .unwrap());
    assert!(!deal_repo
        .is_party_participant(deal_id, Uuid::now_v7())
        .await
        .unwrap());
}

#[sqlx::test(migrations = "../../migrations")]
async fn generates_sequential_references(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool);
    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;

    let first = deal_repo.next_deal_reference().await.unwrap();

    let aggregate = sample_deal_aggregate(supplier, consumer, enhancer, &first);
    deal_repo.create(&aggregate).await.unwrap();

    let second = deal_repo.next_deal_reference().await.unwrap();

    assert_ne!(first, second);
    assert!(first.starts_with("DL-"));
    assert!(second.starts_with("DL-"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn updates_value_totals(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let deal_repo = PostgresDealRepository::new(pool);
    let (supplier, consumer, enhancer) = three_party_fixture(&party_repo).await;

    let aggregate = sample_deal_aggregate(supplier, consumer, enhancer, "DL-2026-0008");
    let deal_id = aggregate.deal.id;
    deal_repo.create(&aggregate).await.unwrap();

    deal_repo
        .update_value_totals(
            deal_id,
            Decimal::from(30000),
            Decimal::from(10),
            Decimal::from(3000),
        )
        .await
        .unwrap();

    let found = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    assert_eq!(found.total_deal_value, Some(Decimal::from(30000)));
    assert_eq!(found.platform_fee_percentage, Decimal::from(10));
    assert_eq!(found.platform_fee_amount, Decimal::from(3000));
}
