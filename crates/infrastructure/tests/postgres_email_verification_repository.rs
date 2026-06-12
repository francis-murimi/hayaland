use domain::entities::{Email, EmailVerification, PasswordHash, User, Username};
use domain::repositories::{EmailVerificationRepository, UserRepository};
use infrastructure::repositories::{PostgresEmailVerificationRepository, PostgresUserRepository};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

fn sample_user(email: &str, username: &str) -> User {
    User::new(
        Uuid::now_v7(),
        Email::new(email).unwrap(),
        Username::new(username).unwrap(),
        PasswordHash::new(format!("hash-{username}")).unwrap(),
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn saves_and_finds_verification(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresEmailVerificationRepository::new(pool);
    let user = sample_user("verify@example.com", "verify_user");
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();

    let verification = EmailVerification::new(
        "token-123",
        user_id,
        OffsetDateTime::now_utc() + time::Duration::hours(24),
    );
    repo.save(&verification).await.unwrap();

    let found = repo.find_by_token("token-123").await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.user_id, user_id);
    assert!(!found.used);
}

#[sqlx::test(migrations = "../../migrations")]
async fn returns_none_for_missing_token(pool: PgPool) {
    let repo = PostgresEmailVerificationRepository::new(pool);

    let found = repo.find_by_token("missing").await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn marks_token_used(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresEmailVerificationRepository::new(pool);
    let user = sample_user("used@example.com", "used_user");

    user_repo.create(&user).await.unwrap();

    let verification = EmailVerification::new(
        "token-used",
        user.id,
        OffsetDateTime::now_utc() + time::Duration::hours(24),
    );
    repo.save(&verification).await.unwrap();
    repo.mark_used("token-used").await.unwrap();

    let found = repo.find_by_token("token-used").await.unwrap().unwrap();
    assert!(found.used);
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalidates_unused_tokens_for_user(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresEmailVerificationRepository::new(pool);
    let user = sample_user("invalidate@example.com", "invalidate_user");

    user_repo.create(&user).await.unwrap();

    let v1 = EmailVerification::new(
        "token-1",
        user.id,
        OffsetDateTime::now_utc() + time::Duration::hours(24),
    );
    let mut v2 = EmailVerification::new(
        "token-2",
        user.id,
        OffsetDateTime::now_utc() + time::Duration::hours(24),
    );
    v2.used = true;

    repo.save(&v1).await.unwrap();
    repo.save(&v2).await.unwrap();
    repo.invalidate_unused_for_user(user.id).await.unwrap();

    assert!(repo.find_by_token("token-1").await.unwrap().unwrap().used);
    assert!(repo.find_by_token("token-2").await.unwrap().unwrap().used);
}
