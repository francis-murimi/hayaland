use infrastructure::config::DatabaseSettings;
use infrastructure::database::create_pool;
use infrastructure::migrations::run_migrations;
use secrecy::SecretString;

#[tokio::test]
async fn create_pool_connects_to_database() {
    let settings = DatabaseSettings {
        url: SecretString::from(
            std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://hayaland@127.0.0.1:5432/hayaland_test".into()),
        ),
        max_connections: 2,
    };

    let pool = create_pool(&settings).await.unwrap();
    let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await.unwrap();
    assert_eq!(row.0, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn run_migrations_succeeds(pool: sqlx::PgPool) {
    run_migrations(&pool).await.unwrap();
}
