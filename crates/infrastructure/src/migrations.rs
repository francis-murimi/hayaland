use anyhow::Context;
use sqlx::PgPool;

/// Run all pending migrations embedded in `migrations/`.
pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .context("failed to run database migrations")
}
