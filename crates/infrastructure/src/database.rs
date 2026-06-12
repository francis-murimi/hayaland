use crate::config::DatabaseSettings;
use anyhow::Context;
use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Build a PostgreSQL connection pool from the supplied settings.
///
/// This function connects eagerly so that startup fails fast if the database
/// is unreachable.
pub async fn create_pool(settings: &DatabaseSettings) -> anyhow::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .connect(settings.url.expose_secret())
        .await
        .context("failed to connect to PostgreSQL")
}
