//! Data retention — manual cleanup when TimescaleDB automatic retention policy
//! is not available (e.g. non-TimescaleDB PostgreSQL or integration tests).
//!
//! In production the TimescaleDB `add_retention_policy` set up in migration 004
//! handles cleanup automatically.  This module provides an on-demand fallback
//! that can be triggered by the CLI (`agentmesh retention run`) or a cron job.

use crate::db::StorePool;

/// Delete events older than `retention_days` days from the `events` table.
///
/// Returns the number of rows deleted.
///
/// This is intentionally a simple DELETE; in a TimescaleDB environment the
/// retention policy drops whole chunks (much faster).  The DELETE here is the
/// fallback path for environments without TimescaleDB or for test fixtures.
pub async fn run_retention_cleanup(
    pool: &StorePool,
    retention_days: u32,
) -> Result<u64, sqlx::Error> {
    tracing::info!(retention_days, "running retention cleanup");

    let result = sqlx::query(
        r#"
        DELETE FROM events
        WHERE timestamp < NOW() - ($1 || ' days')::INTERVAL
        "#,
    )
    .bind(retention_days as i32)
    .execute(pool)
    .await?;

    let deleted = result.rows_affected();
    tracing::info!(deleted, "retention cleanup complete");
    Ok(deleted)
}

/// Return the oldest event timestamp still in the database, or `None` if the
/// table is empty.  Useful for health checks and retention reporting.
pub async fn oldest_event_timestamp(
    pool: &StorePool,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, sqlx::Error> {
    use sqlx::Row;

    let row = sqlx::query("SELECT MIN(timestamp) AS oldest FROM events")
        .fetch_one(pool)
        .await?;

    let ts: Option<chrono::DateTime<chrono::Utc>> = row.try_get("oldest").unwrap_or(None);
    Ok(ts)
}

/// Return the number of events currently stored in the database.
pub async fn event_count(pool: &StorePool) -> Result<i64, sqlx::Error> {
    use sqlx::Row;

    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM events")
        .fetch_one(pool)
        .await?;

    Ok(row.try_get::<i64, _>("cnt").unwrap_or(0))
}

/// Return the approximate number of chunks managed by TimescaleDB for the
/// `events` hypertable.  Returns 0 if TimescaleDB is not installed.
pub async fn chunk_count(pool: &StorePool) -> Result<i64, sqlx::Error> {
    use sqlx::Row;

    // This query works only with TimescaleDB installed; fall back gracefully.
    let result = sqlx::query(
        r#"
        SELECT COUNT(*) AS cnt
        FROM timescaledb_information.chunks
        WHERE hypertable_name = 'events'
        "#,
    )
    .fetch_one(pool)
    .await;

    match result {
        Ok(row) => Ok(row.try_get::<i64, _>("cnt").unwrap_or(0)),
        Err(_) => Ok(0), // TimescaleDB not available — not an error
    }
}

#[cfg(test)]
mod tests {
    // Unit tests for retention logic do not require a live database.
    // Integration tests live in tests/integration/.

    #[test]
    fn retention_days_cast() {
        // Verify the u32 → i32 cast used in the query doesn't panic for
        // typical values.
        let days: u32 = 7;
        let as_i32 = days as i32;
        assert_eq!(as_i32, 7);

        let days_30: u32 = 30;
        assert_eq!(days_30 as i32, 30);
    }
}
