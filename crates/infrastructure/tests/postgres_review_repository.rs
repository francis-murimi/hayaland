use domain::entities::{DealRole, Review, ReviewRating, ReviewText};
use domain::repositories::{ReviewRepository, ReviewSearchCriteria};
use infrastructure::repositories::PostgresReviewRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

async fn setup_deal(pool: &PgPool) -> (Uuid, Uuid, Uuid) {
    let deal_id = Uuid::now_v7();
    let supplier_id = Uuid::now_v7();
    let consumer_id = Uuid::now_v7();
    let category_id = Uuid::now_v7();

    sqlx::query!(
        "INSERT INTO categories (id, category_name, category_code, category_type) VALUES ($1, $2, $3, $4)",
        category_id,
        "Test",
        "TEST",
        "DOMAIN"
    )
        .execute(pool)
        .await
        .unwrap();

    for (id, email) in [
        (supplier_id, "supplier@example.com"),
        (consumer_id, "consumer@example.com"),
    ] {
        sqlx::query!(
            r#"
            INSERT INTO parties (
                id, party_type, display_name, email, verification_status, is_active, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            id,
            "ORGANIZATION",
            "Test Party",
            email,
            "UNVERIFIED",
            true,
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc()
        )
        .execute(pool)
        .await
        .unwrap();
    }

    sqlx::query!(
        r#"
        INSERT INTO deals (
            id, deal_reference, deal_title, domain_category_id, initiator_party_id, initiator_role,
            deal_status, platform_fee_percentage, platform_fee_amount, is_public, current_state_entered_at,
            created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
        deal_id,
        format!("DL-{}-0001", Uuid::now_v7()),
        "Test Deal",
        category_id,
        supplier_id,
        "SUPPLIER",
        "EXECUTING",
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

    for (party_id, role) in [(supplier_id, "SUPPLIER"), (consumer_id, "CONSUMER")] {
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
            party_id == supplier_id,
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc()
        )
        .execute(pool)
        .await
        .unwrap();
    }

    (deal_id, supplier_id, consumer_id)
}

fn sample_review(deal_id: Uuid, reviewer: Uuid, reviewed: Uuid, is_public: bool) -> Review {
    Review::new(
        Uuid::now_v7(),
        deal_id,
        reviewer,
        reviewed,
        DealRole::Consumer,
        ReviewRating::new(4).unwrap(),
        Some(ReviewRating::new(5).unwrap()),
        None,
        None,
        None,
        Some(ReviewText::new("Good partner").unwrap()),
        is_public,
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_review(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id) = setup_deal(&pool).await;
    let repo = PostgresReviewRepository::new(pool);

    let review = sample_review(deal_id, supplier_id, consumer_id, true);
    repo.create(&review).await.unwrap();

    let found = repo.find_by_id(review.id).await.unwrap().unwrap();
    assert_eq!(found.id, review.id);
    assert_eq!(found.overall_rating.value(), 4);
    assert_eq!(found.communication_rating.unwrap().value(), 5);
    assert!(found.is_public);
}

#[sqlx::test(migrations = "../../migrations")]
async fn exists_returns_true_when_present(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id) = setup_deal(&pool).await;
    let repo = PostgresReviewRepository::new(pool);

    let review = sample_review(deal_id, supplier_id, consumer_id, true);
    repo.create(&review).await.unwrap();

    assert!(repo
        .exists(deal_id, supplier_id, consumer_id)
        .await
        .unwrap());
    assert!(!repo
        .exists(deal_id, consumer_id, supplier_id)
        .await
        .unwrap());
}

#[sqlx::test(migrations = "../../migrations")]
async fn count_by_deal(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id) = setup_deal(&pool).await;
    let repo = PostgresReviewRepository::new(pool);

    repo.create(&sample_review(deal_id, supplier_id, consumer_id, true))
        .await
        .unwrap();
    repo.create(&sample_review(deal_id, consumer_id, supplier_id, true))
        .await
        .unwrap();

    assert_eq!(repo.count_by_deal(deal_id).await.unwrap(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_filters_and_paginates(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id) = setup_deal(&pool).await;
    let repo = PostgresReviewRepository::new(pool);

    repo.create(&sample_review(deal_id, supplier_id, consumer_id, true))
        .await
        .unwrap();
    repo.create(&sample_review(deal_id, consumer_id, supplier_id, false))
        .await
        .unwrap();

    let all = repo
        .list(&ReviewSearchCriteria {
            deal_id: Some(deal_id),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.total, 2);

    let public_only = repo
        .list(&ReviewSearchCriteria {
            deal_id: Some(deal_id),
            is_public: Some(true),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(public_only.total, 1);
    assert!(public_only.reviews[0].is_public);

    let by_reviewer = repo
        .list(&ReviewSearchCriteria {
            deal_id: Some(deal_id),
            reviewer_party_id: Some(supplier_id),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_reviewer.total, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_missing_review_pairs(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id) = setup_deal(&pool).await;
    let repo = PostgresReviewRepository::new(pool);

    let missing = repo
        .find_missing_review_pairs(
            deal_id,
            &[
                (supplier_id, DealRole::Supplier),
                (consumer_id, DealRole::Consumer),
            ],
        )
        .await
        .unwrap();
    assert_eq!(missing.len(), 2);

    repo.create(&sample_review(deal_id, supplier_id, consumer_id, true))
        .await
        .unwrap();

    let missing = repo
        .find_missing_review_pairs(
            deal_id,
            &[
                (supplier_id, DealRole::Supplier),
                (consumer_id, DealRole::Consumer),
            ],
        )
        .await
        .unwrap();
    assert_eq!(missing.len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn hide_review(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id) = setup_deal(&pool).await;
    let repo = PostgresReviewRepository::new(pool);

    let review = sample_review(deal_id, supplier_id, consumer_id, true);
    repo.create(&review).await.unwrap();

    repo.hide(review.id, Some("removed".to_string()))
        .await
        .unwrap();

    let hidden = repo.find_by_id(review.id).await.unwrap().unwrap();
    assert!(!hidden.is_public);
    assert!(hidden.review_text.is_none());
    assert_eq!(hidden.platform_response.as_deref(), Some("removed"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_review_fails(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id) = setup_deal(&pool).await;
    let repo = PostgresReviewRepository::new(pool);

    let review = sample_review(deal_id, supplier_id, consumer_id, true);
    repo.create(&review).await.unwrap();

    let duplicate = sample_review(deal_id, supplier_id, consumer_id, true);
    let err = repo.create(&duplicate).await.unwrap_err();
    assert!(matches!(err, domain::errors::DomainError::DuplicateReview));
}
