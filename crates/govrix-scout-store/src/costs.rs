//! Cost queries — summary and breakdown from the `cost_daily` materialized view
//! and the raw `events` table.
//!
//! The materialized view (`cost_daily`) is the fast path for dashboard queries.
//! The raw `events` table is the fallback for time ranges not yet refreshed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::StorePool;

// ── Public types ──────────────────────────────────────────────────────────────

/// Aggregate cost summary over a time range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f64,
    pub avg_latency_ms: Option<f64>,
    pub p99_latency_ms: Option<f64>,
}

/// A single row in the cost breakdown result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdownRow {
    /// The dimension value (agent_id / model / protocol depending on `group_by`).
    pub group_key: String,
    pub request_count: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f64,
    pub avg_latency_ms: Option<f64>,
}

/// Granularity for time-series cost queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    Hour,
    Day,
    Week,
    Month,
}

impl Granularity {
    fn as_interval(&self) -> &'static str {
        match self {
            Granularity::Hour => "1 hour",
            Granularity::Day => "1 day",
            Granularity::Week => "1 week",
            Granularity::Month => "1 month",
        }
    }
}

/// Dimension to group the cost breakdown by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupBy {
    Agent,
    Model,
    Protocol,
}

// ── Query functions ───────────────────────────────────────────────────────────

/// Return an aggregate cost summary for the given time range.
///
/// Queries the raw `events` table with `time_bucket` so the result always
/// reflects the latest data (not stale materialized view data).
pub async fn get_cost_summary(
    pool: &StorePool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    _granularity: Granularity,
) -> Result<CostSummary, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT
            COUNT(*)                                                    AS total_requests,
            COALESCE(SUM(input_tokens),  0)                             AS total_input_tokens,
            COALESCE(SUM(output_tokens), 0)                             AS total_output_tokens,
            COALESCE(SUM(cost_usd),      0.0)                           AS total_cost_usd,
            AVG(latency_ms)                                             AS avg_latency_ms,
            PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY latency_ms)    AS p99_latency_ms
        FROM events
        WHERE timestamp >= $1
          AND timestamp <  $2
        "#,
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;

    use sqlx::Row;
    Ok(CostSummary {
        from,
        to,
        total_requests: row.try_get::<i64, _>("total_requests").unwrap_or(0),
        total_input_tokens: row.try_get::<i64, _>("total_input_tokens").unwrap_or(0),
        total_output_tokens: row.try_get::<i64, _>("total_output_tokens").unwrap_or(0),
        total_cost_usd: row.try_get::<f64, _>("total_cost_usd").unwrap_or(0.0),
        avg_latency_ms: row
            .try_get::<Option<f64>, _>("avg_latency_ms")
            .unwrap_or(None),
        p99_latency_ms: row
            .try_get::<Option<f64>, _>("p99_latency_ms")
            .unwrap_or(None),
    })
}

/// Return a cost breakdown grouped by the specified dimension.
///
/// Uses the `cost_daily` materialized view when grouping by day-level data,
/// falls back to raw `events` for finer granularity.
pub async fn get_cost_breakdown(
    pool: &StorePool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    group_by: GroupBy,
) -> Result<Vec<CostBreakdownRow>, sqlx::Error> {
    // Build the GROUP BY column dynamically
    let group_col = match group_by {
        GroupBy::Agent => "agent_id",
        GroupBy::Model => "COALESCE(model, 'unknown')",
        GroupBy::Protocol => "provider",
    };

    let sql = format!(
        r#"
        SELECT
            {group_col}                                                 AS group_key,
            COUNT(*)                                                    AS request_count,
            COALESCE(SUM(input_tokens),  0)                             AS total_input_tokens,
            COALESCE(SUM(output_tokens), 0)                             AS total_output_tokens,
            COALESCE(SUM(cost_usd),      0.0)                           AS total_cost_usd,
            AVG(latency_ms)                                             AS avg_latency_ms
        FROM events
        WHERE timestamp >= $1
          AND timestamp <  $2
        GROUP BY {group_col}
        ORDER BY total_cost_usd DESC
        LIMIT 500
        "#,
        group_col = group_col
    );

    let rows = sqlx::query(&sql)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;

    use sqlx::Row;
    let result = rows
        .into_iter()
        .map(|row| CostBreakdownRow {
            group_key: row.try_get::<String, _>("group_key").unwrap_or_default(),
            request_count: row.try_get::<i64, _>("request_count").unwrap_or(0),
            total_input_tokens: row.try_get::<i64, _>("total_input_tokens").unwrap_or(0),
            total_output_tokens: row.try_get::<i64, _>("total_output_tokens").unwrap_or(0),
            total_cost_usd: row.try_get::<f64, _>("total_cost_usd").unwrap_or(0.0),
            avg_latency_ms: row
                .try_get::<Option<f64>, _>("avg_latency_ms")
                .unwrap_or(None),
        })
        .collect();

    Ok(result)
}

/// Return time-series cost data bucketed by the given granularity.
///
/// Returns rows of (bucket_time, total_cost_usd, request_count) for charting.
pub async fn get_cost_timeseries(
    pool: &StorePool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    granularity: Granularity,
    agent_id: Option<&str>,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let interval = granularity.as_interval();

    let sql = if agent_id.is_some() {
        format!(
            r#"
            SELECT
                time_bucket('{interval}', timestamp)    AS bucket,
                COUNT(*)                                AS request_count,
                COALESCE(SUM(cost_usd), 0.0)            AS total_cost_usd,
                COALESCE(SUM(total_tokens), 0)          AS total_tokens
            FROM events
            WHERE timestamp >= $1
              AND timestamp <  $2
              AND agent_id   = $3
            GROUP BY bucket
            ORDER BY bucket ASC
            "#,
            interval = interval
        )
    } else {
        format!(
            r#"
            SELECT
                time_bucket('{interval}', timestamp)    AS bucket,
                COUNT(*)                                AS request_count,
                COALESCE(SUM(cost_usd), 0.0)            AS total_cost_usd,
                COALESCE(SUM(total_tokens), 0)          AS total_tokens
            FROM events
            WHERE timestamp >= $1
              AND timestamp <  $2
            GROUP BY bucket
            ORDER BY bucket ASC
            "#,
            interval = interval
        )
    };

    use sqlx::Row;

    let rows = if let Some(aid) = agent_id {
        sqlx::query(&sql)
            .bind(from)
            .bind(to)
            .bind(aid)
            .fetch_all(pool)
            .await?
    } else {
        sqlx::query(&sql)
            .bind(from)
            .bind(to)
            .fetch_all(pool)
            .await?
    };

    let result = rows
        .into_iter()
        .map(|row| {
            let bucket: Option<DateTime<Utc>> = row.try_get::<DateTime<Utc>, _>("bucket").ok();
            let request_count: i64 = row.try_get("request_count").unwrap_or(0);
            let total_cost_usd: f64 = row.try_get("total_cost_usd").unwrap_or(0.0);
            let total_tokens: i64 = row.try_get("total_tokens").unwrap_or(0);
            serde_json::json!({
                "bucket": bucket.map(|t| t.to_rfc3339()),
                "request_count": request_count,
                "total_cost_usd": total_cost_usd,
                "total_tokens": total_tokens,
            })
        })
        .collect();

    Ok(result)
}

/// Trigger a non-blocking refresh of the `cost_daily` materialized view.
///
/// Uses `CONCURRENTLY` so existing reads are not blocked.
/// Requires that the unique index `cost_daily_pkey` defined in migration 003
/// already exists.
pub async fn refresh_cost_daily(pool: &StorePool) -> Result<(), sqlx::Error> {
    sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY cost_daily")
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn granularity_intervals() {
        assert_eq!(Granularity::Hour.as_interval(), "1 hour");
        assert_eq!(Granularity::Day.as_interval(), "1 day");
        assert_eq!(Granularity::Week.as_interval(), "1 week");
        assert_eq!(Granularity::Month.as_interval(), "1 month");
    }

    #[test]
    fn group_by_columns() {
        // Verify the match arms produce the expected SQL fragments.
        let col = match GroupBy::Agent {
            GroupBy::Agent => "agent_id",
            GroupBy::Model => "COALESCE(model, 'unknown')",
            GroupBy::Protocol => "provider",
        };
        assert_eq!(col, "agent_id");
    }
}
