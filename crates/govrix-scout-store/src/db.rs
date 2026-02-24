//! Database connection pool management.
//!
//! Creates and manages a `sqlx::PgPool` configured from `govrix-scout-common::Config`.

use govrix_scout_common::config::DatabaseConfig;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

/// Type alias for the shared connection pool.
pub type StorePool = PgPool;

/// Create a new PostgreSQL connection pool from the given `DatabaseConfig`.
///
/// This function should be called once at startup and the pool shared via `Arc` or
/// injected into handler state.
pub async fn connect(cfg: &DatabaseConfig) -> Result<StorePool, sqlx::Error> {
    tracing::info!(
        url = %cfg.url.replace(
            // Redact password in log output
            cfg.url.split('@').next().unwrap_or(""),
            "[redacted]"
        ),
        max_connections = cfg.max_connections,
        "connecting to PostgreSQL"
    );

    let pool = PgPoolOptions::new()
        .max_connections(cfg.max_connections)
        .min_connections(cfg.min_connections)
        .acquire_timeout(Duration::from_secs(cfg.connect_timeout_secs))
        .connect(&cfg.url)
        .await?;

    tracing::info!("PostgreSQL connection pool established");
    Ok(pool)
}

/// Run a connectivity check against the database.
pub async fn health_check(pool: &StorePool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}
