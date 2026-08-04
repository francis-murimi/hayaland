use domain::entities::trust_score::TrustScoreRow;
use domain::repositories::TrustScoreRepository;
use infrastructure::repositories::PostgresTrustScoreRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

async fn create_user(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO users (id, email, username, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, $4, now(), now())
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

async fn create_party(pool: &PgPool) -> Uuid {
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
    id
}

async fn create_category(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO categories (id, category_name, category_code, category_type) VALUES ($1, $2, $3, $4)",
        id,
        format!("Category {id}"),
        format!("CAT-{id}"),
        "DOMAIN"
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn create_deal(pool: &PgPool, status: &str) -> (Uuid, Uuid, Uuid, Uuid) {
    let deal_id = Uuid::now_v7();
    let supplier_id = create_party(pool).await;
    let consumer_id = create_party(pool).await;
    let enhancer_id = create_party(pool).await;
    let category_id = create_category(pool).await;

    sqlx::query!(
        r#"
        INSERT INTO deals (
            id, deal_reference, deal_title, domain_category_id, initiator_party_id, initiator_role,
            deal_status, platform_fee_percentage, platform_fee_amount, total_deal_value, is_public,
            current_state_entered_at, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
        deal_id,
        format!("DL-{deal_id}"),
        "Test Deal",
        category_id,
        supplier_id,
        "SUPPLIER",
        status,
        Decimal::ZERO,
        Decimal::ZERO,
        Decimal::from(1000),
        false,
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .unwrap();

    for (party_id, role) in [
        (supplier_id, "SUPPLIER"),
        (consumer_id, "CONSUMER"),
        (enhancer_id, "ENHANCER"),
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
            role == "SUPPLIER",
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc()
        )
        .execute(pool)
        .await
        .unwrap();
    }

    (deal_id, supplier_id, consumer_id, enhancer_id)
}

async fn create_review(
    pool: &PgPool,
    deal_id: Uuid,
    reviewer_party_id: Uuid,
    reviewed_party_id: Uuid,
    reviewed_role: &str,
    overall_rating: i32,
    is_public: bool,
) {
    sqlx::query!(
        r#"
        INSERT INTO reviews (
            id, deal_id, reviewer_party_id, reviewed_party_id, reviewed_role,
            overall_rating, communication_rating, reliability_rating, quality_rating,
            timeliness_rating, review_text, is_public, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
        Uuid::now_v7(),
        deal_id,
        reviewer_party_id,
        reviewed_party_id,
        reviewed_role,
        overall_rating,
        Some(4),
        Some(5),
        Some(3),
        Some(4),
        Some("A review."),
        is_public,
        OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn create_dispute(
    pool: &PgPool,
    deal_id: Uuid,
    raised_by_party_id: Uuid,
    raised_by_user_id: Uuid,
    against_party_id: Uuid,
) {
    sqlx::query!(
        r#"
        INSERT INTO disputes (
            id, deal_id, raised_by_party_id, raised_by_user_id, against_party_id,
            dispute_type, dispute_status, resolution_type, resolution_outcome,
            description, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
        Uuid::now_v7(),
        deal_id,
        raised_by_party_id,
        raised_by_user_id,
        against_party_id,
        "QUALITY_ISSUE",
        "RESOLVED",
        Some("MEDIATED"),
        Some("SPLIT"),
        "A dispute.",
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn create_conversation_for_party(pool: &PgPool, party_a_id: Uuid, party_b_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO conversations (id, conversation_type, party_a_id, party_b_id, last_message_at, created_at)
        VALUES ($1, 'DIRECT_PARTY', $2, $3, now(), now())
        "#,
        id,
        party_a_id,
        party_b_id
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn create_message(
    pool: &PgPool,
    conversation_id: Uuid,
    sender_user_id: Uuid,
    sender_party_id: Uuid,
    recipient_party_id: Uuid,
    created_at: OffsetDateTime,
) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO messages (
            id, conversation_id, sender_user_id, sender_party_id, recipient_type,
            recipient_party_id, message_type, content, created_at
        ) VALUES ($1, $2, $3, $4, 'PARTY', $5, 'TEXT', 'hello', $6)
        "#,
        id,
        conversation_id,
        sender_user_id,
        sender_party_id,
        recipient_party_id,
        created_at
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn create_message_read(
    pool: &PgPool,
    message_id: Uuid,
    user_id: Uuid,
    party_id: Uuid,
    read_at: OffsetDateTime,
) {
    sqlx::query!(
        r#"
        INSERT INTO message_reads (id, message_id, user_id, party_id, read_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        Uuid::now_v7(),
        message_id,
        user_id,
        party_id,
        read_at
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_default_and_find_by_party_id(pool: PgPool) {
    let party_id = create_party(&pool).await;
    let repo = PostgresTrustScoreRepository::new(pool);

    repo.create_default(party_id).await.unwrap();

    let found = repo.find_by_party_id(party_id).await.unwrap().unwrap();
    assert_eq!(found.party_id, party_id);
    assert_eq!(found.overall_score, 0.0);
    assert_eq!(found.deals_completed_count, 0);
    assert_eq!(found.profile_completeness, 0.0);
    assert_eq!(found.verification_level, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_by_party_id_returns_none_for_missing(pool: PgPool) {
    let repo = PostgresTrustScoreRepository::new(pool);
    let found = repo.find_by_party_id(Uuid::now_v7()).await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn upsert_inserts_and_updates(pool: PgPool) {
    let party_id = create_party(&pool).await;
    let repo = PostgresTrustScoreRepository::new(pool);

    let mut row = TrustScoreRow::new(party_id);
    row.overall_score = 42.0;
    row.as_supplier_score = Some(40.0);
    row.deals_completed_count = 5;
    row.total_completed_value = 1500.0;
    row.profile_completeness = 0.8;
    row.verification_level = 2;
    row.longevity_days = 30;
    repo.upsert(&row).await.unwrap();

    let found = repo.find_by_party_id(party_id).await.unwrap().unwrap();
    assert_eq!(found.overall_score, 42.0);
    assert_eq!(found.as_supplier_score, Some(40.0));
    assert_eq!(found.deals_completed_count, 5);
    assert_eq!(found.total_completed_value, 1500.0);
    assert_eq!(found.profile_completeness, 0.8);
    assert_eq!(found.verification_level, 2);
    assert_eq!(found.longevity_days, 30);

    let mut updated = found.clone();
    updated.overall_score = 55.0;
    updated.deals_completed_count = 10;
    repo.upsert(&updated).await.unwrap();

    let found = repo.find_by_party_id(party_id).await.unwrap().unwrap();
    assert_eq!(found.overall_score, 55.0);
    assert_eq!(found.deals_completed_count, 10);
}

#[sqlx::test(migrations = "../../migrations")]
async fn increment_counters(pool: PgPool) {
    let party_id = create_party(&pool).await;
    let repo = PostgresTrustScoreRepository::new(pool);

    repo.create_default(party_id).await.unwrap();

    repo.increment_deals_completed_count(party_id, 100.0)
        .await
        .unwrap();
    repo.increment_deals_completed_count(party_id, 250.0)
        .await
        .unwrap();
    repo.increment_deals_cancelled_count(party_id)
        .await
        .unwrap();
    repo.increment_deals_disputed_count(party_id).await.unwrap();
    repo.increment_timeouts_count(party_id).await.unwrap();
    repo.increment_no_shows_count(party_id).await.unwrap();

    let found = repo.find_by_party_id(party_id).await.unwrap().unwrap();
    assert_eq!(found.deals_completed_count, 2);
    assert_eq!(found.total_completed_value, 350.0);
    assert_eq!(found.deals_cancelled_count, 1);
    assert_eq!(found.deals_disputed_count, 1);
    assert_eq!(found.timeouts_count, 1);
    assert_eq!(found.no_shows_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_profile_completeness(pool: PgPool) {
    let party_id = create_party(&pool).await;
    let repo = PostgresTrustScoreRepository::new(pool);

    repo.create_default(party_id).await.unwrap();
    repo.update_profile_completeness(party_id, 0.75)
        .await
        .unwrap();

    let found = repo.find_by_party_id(party_id).await.unwrap().unwrap();
    assert_eq!(found.profile_completeness, 0.75);
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_verification_level(pool: PgPool) {
    let party_id = create_party(&pool).await;
    let repo = PostgresTrustScoreRepository::new(pool);

    repo.create_default(party_id).await.unwrap();
    repo.update_verification_level(party_id, 3).await.unwrap();

    let found = repo.find_by_party_id(party_id).await.unwrap().unwrap();
    assert_eq!(found.verification_level, 3);
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_response_hours(pool: PgPool) {
    let party_id = create_party(&pool).await;
    let repo = PostgresTrustScoreRepository::new(pool);

    repo.create_default(party_id).await.unwrap();
    repo.update_response_hours(party_id, Some(2.5))
        .await
        .unwrap();

    let found = repo.find_by_party_id(party_id).await.unwrap().unwrap();
    assert_eq!(found.average_response_hours, Some(2.5));

    repo.update_response_hours(party_id, None).await.unwrap();
    let found = repo.find_by_party_id(party_id).await.unwrap().unwrap();
    assert_eq!(found.average_response_hours, None);
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_public_cache(pool: PgPool) {
    let party_id = create_party(&pool).await;
    let repo = PostgresTrustScoreRepository::new(pool.clone());

    repo.update_public_cache(party_id, 88.5).await.unwrap();

    let score = sqlx::query_scalar!(r#"SELECT trust_score FROM parties WHERE id = $1"#, party_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(score, 88.5);
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_party_ids_paginates(pool: PgPool) {
    let first = create_party(&pool).await;
    let second = create_party(&pool).await;
    let third = create_party(&pool).await;
    let repo = PostgresTrustScoreRepository::new(pool);

    let page = repo.list_party_ids(2, 0).await.unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(page[0], first);
    assert_eq!(page[1], second);

    let page = repo.list_party_ids(2, 2).await.unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0], third);
}

#[sqlx::test(migrations = "../../migrations")]
async fn compute_response_metrics(pool: PgPool) {
    let recipient_party_id = create_party(&pool).await;
    let sender_party_id = create_party(&pool).await;
    let sender_user_id = create_user(&pool).await;
    let reader_user_id = create_user(&pool).await;
    let repo = PostgresTrustScoreRepository::new(pool.clone());

    let conversation_id =
        create_conversation_for_party(&pool, sender_party_id, recipient_party_id).await;

    let now = OffsetDateTime::now_utc();
    let m1 = create_message(
        &pool,
        conversation_id,
        sender_user_id,
        sender_party_id,
        recipient_party_id,
        now - Duration::hours(10),
    )
    .await;
    let m2 = create_message(
        &pool,
        conversation_id,
        sender_user_id,
        sender_party_id,
        recipient_party_id,
        now - Duration::hours(5),
    )
    .await;
    create_message(
        &pool,
        conversation_id,
        sender_user_id,
        sender_party_id,
        recipient_party_id,
        now - Duration::hours(1),
    )
    .await;

    create_message_read(
        &pool,
        m1,
        reader_user_id,
        recipient_party_id,
        now - Duration::hours(8),
    )
    .await;
    create_message_read(
        &pool,
        m2,
        reader_user_id,
        recipient_party_id,
        now - Duration::hours(3),
    )
    .await;

    let metrics = repo
        .compute_response_metrics(recipient_party_id)
        .await
        .unwrap();
    assert_eq!(metrics.messages_received_90d, 3);
    assert_eq!(metrics.messages_responded_90d, 2);
    let avg = metrics.average_response_hours.unwrap();
    assert!((avg - 2.0).abs() < 0.01, "expected ~2h, got {avg}");
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_role_deal_inputs(pool: PgPool) {
    let (_, supplier_id, _, _) = create_deal(&pool, "COMPLETED").await;
    let (_, _, consumer_id, _) = create_deal(&pool, "CANCELLED").await;
    let repo = PostgresTrustScoreRepository::new(pool);

    let supplier_inputs = repo.find_role_deal_inputs(supplier_id).await.unwrap();
    let supplier_role = supplier_inputs.get("SUPPLIER").unwrap();
    assert_eq!(supplier_role.deals_completed_count, 1);
    assert_eq!(supplier_role.deals_cancelled_count, 0);
    assert_eq!(supplier_role.total_completed_value, 1000.0);

    let consumer_inputs = repo.find_role_deal_inputs(consumer_id).await.unwrap();
    let consumer_role = consumer_inputs.get("CONSUMER").unwrap();
    assert_eq!(consumer_role.deals_completed_count, 0);
    assert_eq!(consumer_role.deals_cancelled_count, 1);
    assert_eq!(consumer_role.total_completed_value, 0.0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_role_reviews(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, enhancer_id) = create_deal(&pool, "COMPLETED").await;
    let repo = PostgresTrustScoreRepository::new(pool.clone());

    create_review(
        &pool,
        deal_id,
        consumer_id,
        supplier_id,
        "SUPPLIER",
        5,
        true,
    )
    .await;
    create_review(
        &pool,
        deal_id,
        enhancer_id,
        supplier_id,
        "SUPPLIER",
        3,
        true,
    )
    .await;
    create_review(
        &pool,
        deal_id,
        supplier_id,
        consumer_id,
        "CONSUMER",
        4,
        false,
    )
    .await;

    let role_reviews = repo.find_role_reviews(supplier_id).await.unwrap();
    let supplier_reviews = role_reviews.get("SUPPLIER").unwrap();
    assert_eq!(supplier_reviews.len(), 2);

    // review_score is the average of the dimension ratings when present.
    for review in supplier_reviews {
        assert_eq!(review.review_score, 4.0);
        assert_eq!(review.deal_value, 1000.0);
    }

    let consumer_reviews = role_reviews.get("CONSUMER");
    assert!(
        consumer_reviews.is_none(),
        "non-public reviews should not appear"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_dispute_inputs(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, _) = create_deal(&pool, "EXECUTING").await;
    let raised_by_user_id = create_user(&pool).await;
    create_dispute(&pool, deal_id, consumer_id, raised_by_user_id, supplier_id).await;
    let repo = PostgresTrustScoreRepository::new(pool);

    let inputs = repo.find_dispute_inputs(supplier_id).await.unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].raised_by_party_id, consumer_id);
    assert_eq!(inputs[0].against_party_id, Some(supplier_id));
    assert_eq!(inputs[0].resolution_type.as_deref(), Some("MEDIATED"));
    assert_eq!(inputs[0].resolution_outcome.as_deref(), Some("SPLIT"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_review_inputs(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, enhancer_id) = create_deal(&pool, "COMPLETED").await;
    let repo = PostgresTrustScoreRepository::new(pool.clone());

    create_review(
        &pool,
        deal_id,
        consumer_id,
        supplier_id,
        "SUPPLIER",
        5,
        true,
    )
    .await;
    create_review(
        &pool,
        deal_id,
        enhancer_id,
        supplier_id,
        "SUPPLIER",
        2,
        false,
    )
    .await;

    let inputs = repo.find_review_inputs(supplier_id).await.unwrap();
    assert_eq!(inputs.len(), 2);

    let public = inputs.iter().find(|r| r.is_public).unwrap();
    assert!(!public.is_hidden);
    assert_eq!(public.review_score, 4.0);
    assert_eq!(public.deal_value, 1000.0);

    let private = inputs.iter().find(|r| !r.is_public).unwrap();
    assert!(private.is_hidden);
    assert_eq!(private.review_score, 4.0);
    assert_eq!(private.deal_value, 1000.0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_account_age_and_activity(pool: PgPool) {
    let party_id = create_party(&pool).await;
    let repo = PostgresTrustScoreRepository::new(pool.clone());

    let (age, activity) = repo.find_account_age_and_activity(party_id).await.unwrap();
    assert!(age >= 0);
    assert_eq!(activity, None);

    let deal_id = Uuid::now_v7();
    let category_id = create_category(&pool).await;
    sqlx::query!(
        r#"
        INSERT INTO deals (
            id, deal_reference, deal_title, domain_category_id, initiator_party_id, initiator_role,
            deal_status, platform_fee_percentage, platform_fee_amount, is_public,
            current_state_entered_at, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
        deal_id,
        format!("DL-{deal_id}"),
        "Test Deal",
        category_id,
        party_id,
        "SUPPLIER",
        "EXECUTING",
        Decimal::ZERO,
        Decimal::ZERO,
        false,
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc()
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        r#"
        INSERT INTO deal_history (id, deal_id, event_type, actor_party_id, details, created_at)
        VALUES ($1, $2, 'STATUS_CHANGED', $3, '{}', now())
        "#,
        Uuid::now_v7(),
        deal_id,
        party_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let (_, activity) = repo.find_account_age_and_activity(party_id).await.unwrap();
    assert_eq!(activity, Some(0));
}
