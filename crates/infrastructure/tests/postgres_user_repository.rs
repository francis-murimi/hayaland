use domain::entities::{Email, PasswordHash, User, Username};
use domain::repositories::UserRepository;
use infrastructure::repositories::PostgresUserRepository;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_user(pool: PgPool) {
    let repo = PostgresUserRepository::new(pool);
    let email = Email::new("repo@example.com").unwrap();
    let username = Username::new("repo_user").unwrap();
    let password_hash = PasswordHash::new("hash".to_string()).unwrap();
    let user = User::new(
        Uuid::now_v7(),
        email.clone(),
        username.clone(),
        password_hash,
    );

    repo.create(&user).await.unwrap();

    let found_by_email = repo.find_by_email(&email).await.unwrap();
    assert!(found_by_email.is_some());
    let found_by_email = found_by_email.unwrap();
    assert_eq!(found_by_email.email, email);
    assert!(found_by_email.is_active);

    let found_by_username = repo.find_by_username(&username).await.unwrap();
    assert!(found_by_username.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_email_is_rejected(pool: PgPool) {
    let repo = PostgresUserRepository::new(pool);
    let email = Email::new("dup@example.com").unwrap();
    let username1 = Username::new("user1").unwrap();
    let username2 = Username::new("user2").unwrap();

    let user1 = User::new(
        Uuid::now_v7(),
        email.clone(),
        username1,
        PasswordHash::new("hash".to_string()).unwrap(),
    );
    repo.create(&user1).await.unwrap();

    let user2 = User::new(
        Uuid::now_v7(),
        email,
        username2,
        PasswordHash::new("hash".to_string()).unwrap(),
    );
    let err = repo.create(&user2).await.unwrap_err();
    assert!(matches!(err, domain::errors::DomainError::DuplicateEmail));
}
