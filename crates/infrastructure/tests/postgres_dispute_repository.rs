use domain::entities::{
    Dispute, DisputeResponse, DisputeSeverity, DisputeStatus, DisputeType, ResolutionOutcome,
    ResolutionType,
};
use domain::repositories::{DisputeFilters, DisputeRepository};
use infrastructure::repositories::PostgresDisputeRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

async fn setup_deal(pool: &PgPool) -> (Uuid, Uuid, Uuid, Uuid) {
    let deal_id = Uuid::now_v7();
    let supplier_id = Uuid::now_v7();
    let consumer_id = Uuid::now_v7();
    let enhancer_id = Uuid::now_v7();
    let category_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();

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

    sqlx::query!(
        r#"
        INSERT INTO users (id, email, username, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        user_id,
        "user@example.com",
        "user_example_com",
        "hashed:password",
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .unwrap();

    for (id, email) in [
        (supplier_id, "supplier@example.com"),
        (consumer_id, "consumer@example.com"),
        (enhancer_id, "enhancer@example.com"),
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
            party_id == supplier_id,
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc()
        )
        .execute(pool)
        .await
        .unwrap();
    }

    (deal_id, supplier_id, consumer_id, user_id)
}

fn sample_dispute(deal_id: Uuid, raised_by: Uuid, raised_by_user: Uuid, against: Uuid) -> Dispute {
    Dispute::new(
        Uuid::now_v7(),
        deal_id,
        raised_by,
        raised_by_user,
        Some(against),
        DisputeType::QualityIssue,
        "Quality was poor.".to_string(),
        vec!["https://example.com/evidence.jpg".to_string()],
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_dispute(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, user_id) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool);

    let dispute = sample_dispute(deal_id, supplier_id, user_id, consumer_id);
    repo.create(&dispute).await.unwrap();

    let found = repo.find_by_id(dispute.id).await.unwrap().unwrap();
    assert_eq!(found.id, dispute.id);
    assert_eq!(found.deal_id, deal_id);
    assert_eq!(found.raised_by_party_id, supplier_id);
    assert_eq!(found.against_party_id, Some(consumer_id));
    assert_eq!(found.dispute_status, DisputeStatus::Open);
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_by_deal_with_pagination(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, user_id) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool);

    let dispute = sample_dispute(deal_id, supplier_id, user_id, consumer_id);
    repo.create(&dispute).await.unwrap();

    let result = repo.list_by_deal(deal_id, 10, 0).await.unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.disputes.len(), 1);
    assert_eq!(result.disputes[0].id, dispute.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_admin_with_filters(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, user_id) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool);

    let dispute = sample_dispute(deal_id, supplier_id, user_id, consumer_id);
    repo.create(&dispute).await.unwrap();

    let filters = DisputeFilters {
        status: Some(DisputeStatus::Open),
        deal_id: Some(deal_id),
        raised_by_party_id: Some(supplier_id),
        against_party_id: Some(consumer_id),
        limit: 10,
        offset: 0,
    };
    let result = repo.list_admin(&filters).await.unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.disputes[0].id, dispute.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn submit_evidence_appends_and_moves_to_under_review(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, user_id) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool);

    let dispute = sample_dispute(deal_id, supplier_id, user_id, consumer_id);
    repo.create(&dispute).await.unwrap();

    repo.submit_evidence(
        dispute.id,
        vec!["https://example.com/more.jpg".to_string()],
        Some("More evidence.".to_string()),
    )
    .await
    .unwrap();

    let found = repo.find_by_id(dispute.id).await.unwrap().unwrap();
    assert_eq!(found.evidence_urls.len(), 2);
    assert_eq!(found.dispute_status, DisputeStatus::UnderReview);
    assert_eq!(found.admin_notes, Some("More evidence.".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn add_and_list_responses(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, user_id) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool);

    let dispute = sample_dispute(deal_id, supplier_id, user_id, consumer_id);
    repo.create(&dispute).await.unwrap();

    let response = DisputeResponse::new(
        Uuid::now_v7(),
        dispute.id,
        consumer_id,
        user_id,
        "We disagree.".to_string(),
    );
    repo.add_response(&response).await.unwrap();

    let responses = repo.list_responses(dispute.id).await.unwrap();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].message, "We disagree.");

    let found = repo.find_by_id(dispute.id).await.unwrap().unwrap();
    assert_eq!(found.dispute_status, DisputeStatus::UnderReview);
}

#[sqlx::test(migrations = "../../migrations")]
async fn escalate_updates_status(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, user_id) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool);

    let dispute = sample_dispute(deal_id, supplier_id, user_id, consumer_id);
    repo.create(&dispute).await.unwrap();

    repo.escalate(dispute.id, user_id, Some("Escalating.".to_string()))
        .await
        .unwrap();

    let found = repo.find_by_id(dispute.id).await.unwrap().unwrap();
    assert_eq!(found.dispute_status, DisputeStatus::Escalated);
}

#[sqlx::test(migrations = "../../migrations")]
async fn resolve_updates_status(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, user_id) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool);

    let dispute = sample_dispute(deal_id, supplier_id, user_id, consumer_id);
    repo.create(&dispute).await.unwrap();

    repo.resolve(
        dispute.id,
        user_id,
        ResolutionType::Mediated,
        ResolutionOutcome::Split,
        DisputeSeverity::Medium,
        Some("Partial refund.".to_string()),
    )
    .await
    .unwrap();

    let found = repo.find_by_id(dispute.id).await.unwrap().unwrap();
    assert_eq!(found.dispute_status, DisputeStatus::Resolved);
    assert_eq!(found.resolution_type, Some(ResolutionType::Mediated));
    assert_eq!(found.resolution_outcome, Some(ResolutionOutcome::Split));
    assert_eq!(found.severity, Some(DisputeSeverity::Medium));
    assert!(found.resolved_at.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn reject_updates_status(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, user_id) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool);

    let dispute = sample_dispute(deal_id, supplier_id, user_id, consumer_id);
    repo.create(&dispute).await.unwrap();

    repo.reject(dispute.id, user_id, "No evidence.".to_string())
        .await
        .unwrap();

    let found = repo.find_by_id(dispute.id).await.unwrap().unwrap();
    assert_eq!(found.dispute_status, DisputeStatus::Rejected);
    assert_eq!(found.resolution_notes, Some("No evidence.".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_open_dispute_fails(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, user_id) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool);

    let dispute = sample_dispute(deal_id, supplier_id, user_id, consumer_id);
    repo.create(&dispute).await.unwrap();

    let duplicate = sample_dispute(deal_id, supplier_id, user_id, consumer_id);
    let err = repo.create(&duplicate).await.unwrap_err();
    assert!(matches!(
        err,
        domain::errors::DomainError::DisputeAlreadyExists
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn increment_deals_disputed_count_upserts(pool: PgPool) {
    let (_, supplier_id, _, _) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool.clone());

    repo.increment_deals_disputed_count(supplier_id)
        .await
        .unwrap();
    repo.increment_deals_disputed_count(supplier_id)
        .await
        .unwrap();

    let row = sqlx::query_scalar!(
        r#"SELECT deals_disputed_count as "count!" FROM trust_scores WHERE party_id = $1"#,
        supplier_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn count_open_by_party_and_against_party(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, user_id) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool);

    let dispute = sample_dispute(deal_id, supplier_id, user_id, consumer_id);
    repo.create(&dispute).await.unwrap();

    let by_party = repo.count_open_by_party(supplier_id).await.unwrap();
    assert_eq!(by_party, 1);

    let against_party = repo.count_open_against_party(consumer_id).await.unwrap();
    assert_eq!(against_party, 1);

    repo.resolve(
        dispute.id,
        user_id,
        ResolutionType::Amicable,
        ResolutionOutcome::InFavorOfRaised,
        DisputeSeverity::Low,
        None,
    )
    .await
    .unwrap();

    assert_eq!(repo.count_open_by_party(supplier_id).await.unwrap(), 0);
    assert_eq!(repo.count_open_against_party(consumer_id).await.unwrap(), 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn resolve_and_reject_missing_dispute_fails(pool: PgPool) {
    let (_, _, _, user_id) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool);
    let missing_id = Uuid::now_v7();

    let resolve_err = repo
        .resolve(
            missing_id,
            user_id,
            ResolutionType::Amicable,
            ResolutionOutcome::InFavorOfRaised,
            DisputeSeverity::Low,
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(
        resolve_err,
        domain::errors::DomainError::DisputeNotFound
    ));

    let reject_err = repo
        .reject(missing_id, user_id, "No evidence.".to_string())
        .await
        .unwrap_err();
    assert!(matches!(
        reject_err,
        domain::errors::DomainError::DisputeNotFound
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_dispute_with_resolution_fields(pool: PgPool) {
    let (deal_id, supplier_id, consumer_id, user_id) = setup_deal(&pool).await;
    let repo = PostgresDisputeRepository::new(pool);

    let mut dispute = sample_dispute(deal_id, supplier_id, user_id, consumer_id);
    dispute
        .resolve(
            ResolutionType::Arbitrated,
            ResolutionOutcome::Split,
            DisputeSeverity::High,
            Some("Arbitration completed.".to_string()),
            user_id,
        )
        .unwrap();

    repo.create(&dispute).await.unwrap();

    let found = repo.find_by_id(dispute.id).await.unwrap().unwrap();
    assert_eq!(found.dispute_status, DisputeStatus::Resolved);
    assert_eq!(found.resolution_type, Some(ResolutionType::Arbitrated));
    assert_eq!(found.resolution_outcome, Some(ResolutionOutcome::Split));
    assert_eq!(found.severity, Some(DisputeSeverity::High));
    assert_eq!(found.resolved_by_user_id, Some(user_id));
}
