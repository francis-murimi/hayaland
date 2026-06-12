use domain::entities::{Email, PasswordHash, PasswordResetToken, User, Username};
use domain::repositories::{PasswordResetRepository, UserRepository};
use infrastructure::repositories::{PostgresPasswordResetRepository, PostgresUserRepository};
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
async fn saves_and_finds_token(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresPasswordResetRepository::new(pool);
    let user = sample_user("reset@example.com", "reset_user");
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();

    let token = PasswordResetToken::new(
        "token-123",
        user_id,
        OffsetDateTime::now_utc() + time::Duration::hours(1),
    );
    repo.save(&token).await.unwrap();

    let found = repo.find_by_token("token-123").await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.user_id, user_id);
    assert!(!found.used);
}

#[sqlx::test(migrations = "../../migrations")]
async fn returns_none_for_missing_token(pool: PgPool) {
    let repo = PostgresPasswordResetRepository::new(pool);

    let found = repo.find_by_token("missing").await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn marks_token_used(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresPasswordResetRepository::new(pool);
    let user = sample_user("used@example.com", "used_user");

    user_repo.create(&user).await.unwrap();

    let token = PasswordResetToken::new(
        "token-used",
        user.id,
        OffsetDateTime::now_utc() + time::Duration::hours(1),
    );
    repo.save(&token).await.unwrap();
    repo.mark_used("token-used").await.unwrap();

    let found = repo.find_by_token("token-used").await.unwrap().unwrap();
    assert!(found.used);
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalidates_unused_tokens_for_user(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresPasswordResetRepository::new(pool);
    let user = sample_user("invalidate@example.com", "invalidate_user");

    user_repo.create(&user).await.unwrap();

    let t1 = PasswordResetToken::new(
        "token-1",
        user.id,
        OffsetDateTime::now_utc() + time::Duration::hours(1),
    );
    let mut t2 = PasswordResetToken::new(
        "token-2",
        user.id,
        OffsetDateTime::now_utc() + time::Duration::hours(1),
    );
    t2.used = true;

    repo.save(&t1).await.unwrap();
    repo.save(&t2).await.unwrap();
    repo.invalidate_unused_for_user(user.id).await.unwrap();

    assert!(repo.find_by_token("token-1").await.unwrap().unwrap().used);
    assert!(repo.find_by_token("token-2").await.unwrap().unwrap().used);
}
