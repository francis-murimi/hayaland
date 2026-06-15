use domain::entities::{
    Party, PartyType, PartyVerification, PartyVerificationStatus, PartyVerificationType,
};
use domain::repositories::{PartyRepository, PartyVerificationRepository, VerificationListFilters};
use infrastructure::repositories::{PostgresPartyRepository, PostgresPartyVerificationRepository};
use sqlx::{PgPool, Postgres};

use time::OffsetDateTime;
use uuid::Uuid;

async fn seeded_party(pool: &PgPool) -> Uuid {
    let repo = PostgresPartyRepository::new(pool.clone());
    let party_id = Uuid::now_v7();
    let party = Party::new(
        party_id,
        PartyType::Organization,
        domain::entities::DisplayName::new("Test Party").unwrap(),
        domain::entities::Email::new("test@example.com").unwrap(),
    );
    repo.create(&party).await.unwrap();
    party_id
}

async fn seeded_admin(pool: &PgPool) -> Uuid {
    seeded_user_with_email(pool, "admin@example.com", "admin").await
}

async fn seeded_user(pool: &PgPool) -> Uuid {
    seeded_user_with_email(pool, "user@example.com", "user").await
}

async fn seeded_user_with_email(pool: &PgPool, email: &str, username: &str) -> Uuid {
    let user_id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO users (id, email, username, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        user_id,
        email,
        username,
        "hashed:password123",
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .unwrap();
    user_id
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_and_find_by_id(pool: PgPool) {
    let party_id = seeded_party(&pool).await;
    let user_id = seeded_user(&pool).await;
    let repo = PostgresPartyVerificationRepository::new(pool.clone());
    let verification = PartyVerification::new(
        Uuid::now_v7(),
        party_id,
        user_id,
        PartyVerificationType::GovernmentId,
        vec!["url".to_string()],
        None,
    );

    repo.create(&verification).await.unwrap();
    let found = repo.find_by_id(verification.id).await.unwrap().unwrap();

    assert_eq!(found.id, verification.id);
    assert_eq!(found.party_id, party_id);
    assert_eq!(found.verification_type, PartyVerificationType::GovernmentId);
    assert_eq!(found.status, PartyVerificationStatus::Pending);
    assert_eq!(found.points, 30);
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_active_by_party_and_type(pool: PgPool) {
    let party_id = seeded_party(&pool).await;
    let user_id = seeded_user(&pool).await;
    let repo = PostgresPartyVerificationRepository::new(pool.clone());
    let verification = PartyVerification::new(
        Uuid::now_v7(),
        party_id,
        user_id,
        PartyVerificationType::Email,
        vec![],
        None,
    );

    repo.create(&verification).await.unwrap();

    let found = repo
        .find_active_by_party_and_type(party_id, PartyVerificationType::Email)
        .await
        .unwrap();
    assert!(found.is_some());

    let not_found = repo
        .find_active_by_party_and_type(party_id, PartyVerificationType::Phone)
        .await
        .unwrap();
    assert!(not_found.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_active_type_is_rejected(pool: PgPool) {
    let party_id = seeded_party(&pool).await;
    let user_id = seeded_user(&pool).await;
    let repo = PostgresPartyVerificationRepository::new(pool.clone());
    let v1 = PartyVerification::new(
        Uuid::now_v7(),
        party_id,
        user_id,
        PartyVerificationType::Email,
        vec![],
        None,
    );
    let v2 = PartyVerification::new(
        Uuid::now_v7(),
        party_id,
        user_id,
        PartyVerificationType::Email,
        vec![],
        None,
    );

    repo.create(&v1).await.unwrap();
    let err = repo.create(&v2).await.unwrap_err();
    assert!(matches!(
        err,
        domain::errors::DomainError::DuplicateVerification
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn approve_reject_revoke_lifecycle(pool: PgPool) {
    let party_id = seeded_party(&pool).await;
    let user_id = seeded_user(&pool).await;
    let repo = PostgresPartyVerificationRepository::new(pool.clone());
    let admin_id = seeded_admin(&pool).await;
    let verification = PartyVerification::new(
        Uuid::now_v7(),
        party_id,
        user_id,
        PartyVerificationType::GovernmentId,
        vec!["url".to_string()],
        None,
    );

    repo.create(&verification).await.unwrap();

    repo.approve(verification.id, admin_id, Some("ok".to_string()))
        .await
        .unwrap();
    let approved = repo.find_by_id(verification.id).await.unwrap().unwrap();
    assert_eq!(approved.status, PartyVerificationStatus::Approved);
    assert_eq!(approved.reviewed_by_user_id, Some(admin_id));

    let err = repo
        .reject(verification.id, admin_id, "too late".to_string(), None)
        .await
        .unwrap_err();
    assert!(!matches!(
        err,
        domain::errors::DomainError::DuplicateVerification
    ));

    repo.revoke(verification.id, admin_id, "fraud".to_string(), None)
        .await
        .unwrap();
    let revoked = repo.find_by_id(verification.id).await.unwrap().unwrap();
    assert_eq!(revoked.status, PartyVerificationStatus::Revoked);
}

#[sqlx::test(migrations = "../../migrations")]
async fn sum_approved_points_excludes_expired(pool: PgPool) {
    let party_id = seeded_party(&pool).await;
    let user_id = seeded_user(&pool).await;
    let repo = PostgresPartyVerificationRepository::new(pool.clone());
    let admin_id = seeded_admin(&pool).await;

    let email = PartyVerification::new(
        Uuid::now_v7(),
        party_id,
        user_id,
        PartyVerificationType::Email,
        vec![],
        None,
    );
    repo.create(&email).await.unwrap();
    repo.approve(email.id, admin_id, None).await.unwrap();

    let phone = PartyVerification::new(
        Uuid::now_v7(),
        party_id,
        user_id,
        PartyVerificationType::Phone,
        vec![],
        None,
    );
    repo.create(&phone).await.unwrap();
    repo.approve(phone.id, admin_id, None).await.unwrap();

    let mut expired = PartyVerification::new(
        Uuid::now_v7(),
        party_id,
        user_id,
        PartyVerificationType::GovernmentId,
        vec!["url".to_string()],
        None,
    );
    expired.expires_at = Some(OffsetDateTime::now_utc() - time::Duration::hours(1));
    repo.create(&expired).await.unwrap();
    repo.approve(expired.id, admin_id, None).await.unwrap();

    let sum = repo.sum_approved_points(party_id).await.unwrap();
    assert_eq!(sum, 25); // email + phone only
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_filters_by_status(pool: PgPool) {
    let party_id = seeded_party(&pool).await;
    let user_id = seeded_user(&pool).await;
    let repo = PostgresPartyVerificationRepository::new(pool.clone());
    let admin_id = seeded_admin(&pool).await;

    let v1 = PartyVerification::new(
        Uuid::now_v7(),
        party_id,
        user_id,
        PartyVerificationType::Email,
        vec![],
        None,
    );
    repo.create(&v1).await.unwrap();
    repo.approve(v1.id, admin_id, None).await.unwrap();

    let v2 = PartyVerification::new(
        Uuid::now_v7(),
        party_id,
        user_id,
        PartyVerificationType::Phone,
        vec![],
        None,
    );
    repo.create(&v2).await.unwrap();

    let pending = repo
        .list(&VerificationListFilters {
            status: Some("PENDING".to_string()),
            limit: 10,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(pending.total, 1);
    assert_eq!(
        pending.verifications[0].verification_type,
        PartyVerificationType::Phone
    );

    let approved = repo
        .list(&VerificationListFilters {
            status: Some("APPROVED".to_string()),
            limit: 10,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(approved.total, 1);
    assert_eq!(
        approved.verifications[0].verification_type,
        PartyVerificationType::Email
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_verification_level_upserts_trust_scores(pool: PgPool) {
    let party_id = seeded_party(&pool).await;
    let repo = PostgresPartyVerificationRepository::new(pool.clone());

    repo.update_verification_level(party_id, 4).await.unwrap();
    let row: i32 = sqlx::query_scalar::<Postgres, i32>(
        "SELECT verification_level FROM trust_scores WHERE party_id = $1",
    )
    .bind(party_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row, 4);

    repo.update_verification_level(party_id, 5).await.unwrap();
    let row: i32 = sqlx::query_scalar::<Postgres, i32>(
        "SELECT verification_level FROM trust_scores WHERE party_id = $1",
    )
    .bind(party_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row, 5);
}
