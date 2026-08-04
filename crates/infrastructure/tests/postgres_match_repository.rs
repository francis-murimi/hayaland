use domain::entities::{
    MatchGeneratedBy, MatchScoreBreakdown, MatchScoreWeights, MatchStatus, MatchSuggestion,
};
use domain::repositories::{MatchFilters, MatchRepository};
use infrastructure::repositories::PostgresMatchRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

async fn create_category(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO categories (id, category_name, category_code, category_type) VALUES ($1, $2, $3, $4)",
        id,
        "Test",
        "TEST",
        "DOMAIN"
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn create_minimal_deal(
    pool: &PgPool,
    supplier: Uuid,
    consumer: Uuid,
    enhancer: Uuid,
) -> Uuid {
    let category_id = create_category(pool).await;
    let deal_id = Uuid::now_v7();
    let reference = format!("DL-{}", Uuid::now_v7());
    sqlx::query!(
        r#"
        INSERT INTO deals (
            id, deal_reference, deal_title, domain_category_id, initiator_party_id, initiator_role,
            deal_status, platform_fee_percentage, platform_fee_amount, is_public, current_state_entered_at,
            created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
        deal_id,
        reference,
        "Test Deal",
        category_id,
        supplier,
        "SUPPLIER",
        "DRAFT",
        Decimal::ZERO,
        Decimal::ZERO,
        false,
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc()
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
                id, deal_id, party_id, role, participation_status, is_initiator, invited_at, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            Uuid::now_v7(),
            deal_id,
            party_id,
            role,
            "ACCEPTED",
            party_id == supplier,
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc()
        )
        .execute(pool)
        .await
        .unwrap();
    }

    deal_id
}

async fn create_party(pool: &PgPool, suffix: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO parties (
            id, party_type, display_name, email, verification_status, is_active, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        id,
        "ORGANIZATION",
        format!("Party {suffix}"),
        format!("party-{suffix}@example.com"),
        "UNVERIFIED",
        true,
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

fn sample_match(supplier: Uuid, consumer: Uuid, enhancer: Uuid, score: f64) -> MatchSuggestion {
    let mut suggestion = MatchSuggestion::new(
        Uuid::now_v7(),
        supplier,
        consumer,
        enhancer,
        MatchScoreBreakdown::new(
            [score, score, score, score, score, score, score],
            MatchScoreWeights::default(),
        ),
        "test match".into(),
    )
    .unwrap();
    suggestion.generated_by = MatchGeneratedBy::Algorithm;
    suggestion.set_suggested_deal_value(Decimal::from(1000));
    suggestion
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_match(pool: PgPool) {
    let supplier = create_party(&pool, "supplier").await;
    let consumer = create_party(&pool, "consumer").await;
    let enhancer = create_party(&pool, "enhancer").await;
    let repo = PostgresMatchRepository::new(pool);

    let suggestion = sample_match(supplier, consumer, enhancer, 0.8);
    let id = suggestion.id;

    repo.create(&suggestion).await.unwrap();

    let found = repo.find_by_id(id).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, id);
    assert_eq!(found.match_status, MatchStatus::Pending);
}

#[sqlx::test(migrations = "../../migrations")]
async fn lists_for_party(pool: PgPool) {
    let supplier = create_party(&pool, "supplier").await;
    let consumer = create_party(&pool, "consumer").await;
    let enhancer = create_party(&pool, "enhancer").await;
    let repo = PostgresMatchRepository::new(pool);

    let suggestion = sample_match(supplier, consumer, enhancer, 0.8);
    repo.create(&suggestion).await.unwrap();

    let results = repo
        .list_for_party(supplier, None, &MatchFilters::default())
        .await
        .unwrap();
    assert_eq!(results.len(), 1);

    let results = repo
        .list_for_party(consumer, None, &MatchFilters::default())
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn filters_by_status_and_score(pool: PgPool) {
    let supplier = create_party(&pool, "supplier").await;
    let consumer = create_party(&pool, "consumer").await;
    let enhancer = create_party(&pool, "enhancer").await;
    let repo = PostgresMatchRepository::new(pool);

    let suggestion = sample_match(supplier, consumer, enhancer, 0.9);
    repo.create(&suggestion).await.unwrap();

    repo.update_status(
        suggestion.id,
        MatchStatus::Accepted,
        Some("accepted".into()),
    )
    .await
    .unwrap();

    let filters = MatchFilters {
        status: Some(MatchStatus::Accepted),
        ..Default::default()
    };
    let results = repo.list_for_party(supplier, None, &filters).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].match_status, MatchStatus::Accepted);

    let filters = MatchFilters {
        min_score: Some(0.95),
        ..Default::default()
    };
    let results = repo.list_for_party(supplier, None, &filters).await.unwrap();
    assert!(results.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn counter_proposal_and_conversion(pool: PgPool) {
    let supplier = create_party(&pool, "supplier").await;
    let consumer = create_party(&pool, "consumer").await;
    let enhancer = create_party(&pool, "enhancer").await;
    let deal_id = create_minimal_deal(&pool, supplier, consumer, enhancer).await;
    let repo = PostgresMatchRepository::new(pool);

    let suggestion = sample_match(supplier, consumer, enhancer, 0.8);
    repo.create(&suggestion).await.unwrap();

    repo.update_counter_proposal(
        suggestion.id,
        Some(Decimal::from(2000)),
        Some("counter".into()),
    )
    .await
    .unwrap();

    let found = repo.find_by_id(suggestion.id).await.unwrap().unwrap();
    assert_eq!(found.match_status, MatchStatus::CounterProposed);
    assert_eq!(found.suggested_deal_value, Some(Decimal::from(2000)));
    assert!(found.responded_at.is_some());

    repo.set_converted_deal(suggestion.id, deal_id)
        .await
        .unwrap();
    let found = repo.find_by_id(suggestion.id).await.unwrap().unwrap();
    assert_eq!(found.match_status, MatchStatus::ConvertedToDeal);
    assert_eq!(found.converted_deal_id, Some(deal_id));
}

#[sqlx::test(migrations = "../../migrations")]
async fn deletes_by_party_and_status(pool: PgPool) {
    let supplier = create_party(&pool, "supplier").await;
    let consumer = create_party(&pool, "consumer").await;
    let enhancer = create_party(&pool, "enhancer").await;
    let repo = PostgresMatchRepository::new(pool);

    let suggestion = sample_match(supplier, consumer, enhancer, 0.8);
    repo.create(&suggestion).await.unwrap();

    let deleted = repo
        .delete_by_party(supplier, Some(MatchStatus::Pending))
        .await
        .unwrap();
    assert_eq!(deleted, 1);

    let found = repo.find_by_id(suggestion.id).await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn counts_by_status(pool: PgPool) {
    let supplier = create_party(&pool, "supplier").await;
    let consumer = create_party(&pool, "consumer").await;
    let enhancer = create_party(&pool, "enhancer").await;
    let repo = PostgresMatchRepository::new(pool);

    let suggestion = sample_match(supplier, consumer, enhancer, 0.8);
    repo.create(&suggestion).await.unwrap();
    repo.update_status(suggestion.id, MatchStatus::Accepted, None)
        .await
        .unwrap();

    let counts = repo.count_by_status(supplier).await.unwrap();
    assert_eq!(counts.accepted, 1);
    assert_eq!(counts.pending, 0);

    let all_counts = repo.count_all_by_status().await.unwrap();
    assert_eq!(all_counts.accepted, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn finds_existing_pending(pool: PgPool) {
    let supplier = create_party(&pool, "supplier").await;
    let consumer = create_party(&pool, "consumer").await;
    let enhancer = create_party(&pool, "enhancer").await;
    let repo = PostgresMatchRepository::new(pool);

    let suggestion = sample_match(supplier, consumer, enhancer, 0.8);
    repo.create(&suggestion).await.unwrap();

    let found = repo
        .find_existing_pending(supplier, consumer, enhancer)
        .await
        .unwrap();
    assert!(found.is_some());

    repo.update_status(suggestion.id, MatchStatus::Accepted, None)
        .await
        .unwrap();
    let found = repo
        .find_existing_pending(supplier, consumer, enhancer)
        .await
        .unwrap();
    assert!(found.is_none());
}
