use domain::entities::{Email, PasswordHash, User, Username};
use domain::errors::DomainError;
use domain::repositories::UserRepository;
use infrastructure::repositories::PostgresUserRepository;
use sqlx::PgPool;
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
async fn creates_and_finds_user_by_email(pool: PgPool) {
    let repo = PostgresUserRepository::new(pool);
    let user = sample_user("repo@example.com", "repo_user");
    let email = user.email.clone();

    repo.create(&user).await.unwrap();

    let found = repo.find_by_email(&email).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.email, email);
    assert!(found.is_active);
    assert!(found.has_role("user"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn finds_user_by_id(pool: PgPool) {
    let repo = PostgresUserRepository::new(pool);
    let user = sample_user("byid@example.com", "byid");
    let id = user.id;

    repo.create(&user).await.unwrap();

    let found = repo.find_by_id(id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn returns_none_for_missing_user(pool: PgPool) {
    let repo = PostgresUserRepository::new(pool);

    let found = repo.find_by_id(Uuid::now_v7()).await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn lists_users_with_active_filter(pool: PgPool) {
    let repo = PostgresUserRepository::new(pool);
    let mut inactive = sample_user("inactive@example.com", "inactive");
    inactive.is_active = false;

    repo.create(&sample_user("active@example.com", "active"))
        .await
        .unwrap();
    repo.create(&inactive).await.unwrap();

    let active = repo.list(10, 0, Some(true)).await.unwrap();
    assert_eq!(active.len(), 1);
    assert!(active[0].is_active);

    let inactive = repo.list(10, 0, Some(false)).await.unwrap();
    assert_eq!(inactive.len(), 1);
    assert!(!inactive[0].is_active);

    let all = repo.list(10, 0, None).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn updates_user(pool: PgPool) {
    let repo = PostgresUserRepository::new(pool);
    let user = sample_user("update@example.com", "update_user");
    let id = user.id;

    repo.create(&user).await.unwrap();

    let mut updated = repo.find_by_id(id).await.unwrap().unwrap();
    updated.email = Email::new("updated@example.com").unwrap();
    updated.username = Username::new("updated_user").unwrap();

    repo.update(&updated).await.unwrap();

    let found = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(found.email.as_str(), "updated@example.com");
    assert_eq!(found.username.as_str(), "updated_user");
}

#[sqlx::test(migrations = "../../migrations")]
async fn deactivates_user(pool: PgPool) {
    let repo = PostgresUserRepository::new(pool);
    let user = sample_user("deactivate@example.com", "deactivate_user");
    let id = user.id;

    repo.create(&user).await.unwrap();

    let mut updated = repo.find_by_id(id).await.unwrap().unwrap();
    updated.is_active = false;
    repo.update(&updated).await.unwrap();

    let found = repo.find_by_id(id).await.unwrap().unwrap();
    assert!(!found.is_active);
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_email_is_rejected(pool: PgPool) {
    let repo = PostgresUserRepository::new(pool);
    let first = sample_user("dup@example.com", "user1");
    let mut second = sample_user("other@example.com", "user2");
    second.email = first.email.clone();

    repo.create(&first).await.unwrap();
    let err = repo.create(&second).await.unwrap_err();
    assert!(matches!(err, DomainError::DuplicateEmail));
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_username_is_rejected(pool: PgPool) {
    let repo = PostgresUserRepository::new(pool);
    let first = sample_user("first@example.com", "shared");
    let mut second = sample_user("second@example.com", "shared2");
    second.username = first.username.clone();

    repo.create(&first).await.unwrap();
    let err = repo.create(&second).await.unwrap_err();
    assert!(matches!(err, DomainError::DuplicateUsername));
}
