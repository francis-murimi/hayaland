use domain::entities::Role;
use domain::repositories::RoleRepository;
use infrastructure::repositories::PostgresRoleRepository;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn saves_and_finds_role(pool: PgPool) {
    let repo = PostgresRoleRepository::new(pool);
    let role = Role::new(
        "deal_manager",
        vec!["deals:read".to_string(), "deals:write".to_string()],
    );

    repo.save(&role).await.unwrap();

    let found = repo.find_by_name("deal_manager").await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.name, "deal_manager");
    assert_eq!(found.scopes, vec!["deals:read", "deals:write"]);
    assert!(!found.is_builtin);
}

#[sqlx::test(migrations = "../../migrations")]
async fn returns_none_for_missing_role(pool: PgPool) {
    let repo = PostgresRoleRepository::new(pool);

    let found = repo.find_by_name("does_not_exist").await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn lists_roles(pool: PgPool) {
    let repo = PostgresRoleRepository::new(pool);
    let custom = Role::builtin("custom", vec!["custom:read".to_string()]);

    repo.save(&custom).await.unwrap();

    let roles = repo.list().await.unwrap();
    assert!(!roles.is_empty());
    assert!(roles.iter().any(|r| r.name == "custom"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn updates_existing_role(pool: PgPool) {
    let repo = PostgresRoleRepository::new(pool);
    let mut role = Role::builtin("updatable", vec!["a".to_string()]);

    repo.save(&role).await.unwrap();
    role.scopes = vec!["a".to_string(), "b".to_string()];
    repo.save(&role).await.unwrap();

    let found = repo.find_by_name("updatable").await.unwrap().unwrap();
    assert_eq!(found.scopes, vec!["a", "b"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn deletes_non_builtin_role(pool: PgPool) {
    let repo = PostgresRoleRepository::new(pool);
    let role = Role::new("deletable", vec!["x".to_string()]);

    repo.save(&role).await.unwrap();
    repo.delete("deletable").await.unwrap();

    let found = repo.find_by_name("deletable").await.unwrap();
    assert!(found.is_none());
}
