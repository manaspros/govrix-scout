//! Trace CRUD and lifecycle management — maps to the `traces` table (migration 011).
//!
//! Traces are created lazily on the first event for a new session.
//! They are updated asynchronously in the background writer:
//!   - On llm.response / tool.result: update cost and peak risk.
//!   - On session.end or kill switch: mark completed/stopped.
//!   - On error with no subsequent events within 60s: mark failed.

use chrono::{DateTime, Utc};
use govrix_scout_common::models::trace::Trace;
use sqlx::Row;
use uuid::Uuid;

use crate::db::StorePool;

// ── Write path ────────────────────────────────────────────────────────────────

/// Insert a new trace record.
///
/// Returns `Err` on conflict — callers should check before inserting or use
/// `create_trace_if_not_exists` instead.
pub async fn create_trace(pool: &StorePool, trace: &Trace) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO traces (
            trace_id, root_agent_id, task_description, status,
            stopped_by, error_message,
            started_at, completed_at, created_at,
            total_cost_usd, peak_risk_score, event_count, agent_count,
            external_trace_id, metadata
        ) VALUES (
            $1, $2, $3, $4,
            $5, $6,
            $7, $8, $9,
            $10, $11, $12, $13,
            $14, $15
        )
        "#,
    )
    .bind(trace.trace_id)
    .bind(&trace.root_agent_id)
    .bind(&trace.task_description)
    .bind(trace.status.to_string())
    .bind(&trace.stopped_by)
    .bind(&trace.error_message)
    .bind(trace.started_at)
    .bind(trace.completed_at)
    .bind(trace.created_at)
    .bind(trace.total_cost_usd)
    .bind(trace.peak_risk_score)
    .bind(trace.event_count)
    .bind(trace.agent_count)
    .bind(&trace.external_trace_id)
    .bind(&trace.metadata)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update a trace's cost and peak risk score.
///
/// Called by the background writer after each llm.response or tool.result event.
/// `cost_delta` is added to `total_cost_usd`.
/// `peak_risk` replaces `peak_risk_score` only if it is higher than the current value.
pub async fn update_trace_stats(
    pool: &StorePool,
    trace_id: Uuid,
    cost_delta: f64,
    peak_risk: Option<f32>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE traces SET
            total_cost_usd  = total_cost_usd + $2,
            event_count     = event_count + 1,
            peak_risk_score = GREATEST(peak_risk_score, $3)
        WHERE trace_id = $1
        "#,
    )
    .bind(trace_id)
    .bind(cost_delta)
    .bind(peak_risk)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update a trace's stats from a whole batch (multiple events at once).
///
/// More efficient than calling `update_trace_stats` once per event.
/// `event_count_delta` is the total number of events in this batch for this trace.
pub async fn update_trace_stats_batch(
    pool: &StorePool,
    trace_id: Uuid,
    cost_delta: f64,
    peak_risk: Option<f32>,
    event_count_delta: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE traces SET
            total_cost_usd  = total_cost_usd + $2,
            event_count     = event_count + $3,
            peak_risk_score = GREATEST(peak_risk_score, $4)
        WHERE trace_id = $1
        "#,
    )
    .bind(trace_id)
    .bind(cost_delta)
    .bind(event_count_delta)
    .bind(peak_risk)
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a trace as completed (or stopped/failed).
pub async fn complete_trace(
    pool: &StorePool,
    trace_id: Uuid,
    status: &str,
    stopped_by: Option<&str>,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE traces SET
            status        = $2,
            stopped_by    = $3,
            error_message = $4,
            completed_at  = NOW()
        WHERE trace_id = $1
        "#,
    )
    .bind(trace_id)
    .bind(status)
    .bind(stopped_by)
    .bind(error_message)
    .execute(pool)
    .await?;

    Ok(())
}

// ── Read path ─────────────────────────────────────────────────────────────────

/// Get a trace by its UUID.
pub async fn get_trace(
    pool: &StorePool,
    trace_id: Uuid,
) -> Result<Option<serde_json::Value>, sqlx::Error> {

    let row = sqlx::query(
        r#"
        SELECT
            trace_id, root_agent_id, task_description, status,
            stopped_by, error_message,
            started_at, completed_at, created_at,
            total_cost_usd, peak_risk_score, event_count, agent_count,
            external_trace_id, metadata
        FROM traces
        WHERE trace_id = $1
        LIMIT 1
        "#,
    )
    .bind(trace_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| row_to_json(&r)))
}

/// Get all events belonging to a trace, ordered by timestamp ASC.
///
/// Returns raw JSONB rows — includes the new tracing columns (span_id,
/// parent_span_id, event_kind, etc.) added in migration 009.
pub async fn get_trace_spans(
    pool: &StorePool,
    trace_id: Uuid,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {

    let rows = sqlx::query(
        r#"
        SELECT
            id, session_id, agent_id,
            timestamp, latency_ms,
            direction, method, upstream_target, provider, model,
            status_code, finish_reason, raw_size_bytes,
            input_tokens, output_tokens, total_tokens, cost_usd,
            lineage_hash, compliance_tag, tags, error_message,
            created_at,
            event_kind, span_id, parent_span_id, trace_id,
            tool_name, tool_args, tool_result, mcp_server,
            risk_score, external_trace_id
        FROM events
        WHERE trace_id = $1
        ORDER BY timestamp ASC
        "#,
    )
    .bind(trace_id)
    .fetch_all(pool)
    .await?;

    let result = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id":               r.try_get::<Uuid, _>("id").ok().map(|u| u.to_string()),
                "session_id":       r.try_get::<Uuid, _>("session_id").ok().map(|u| u.to_string()),
                "agent_id":         r.try_get::<String, _>("agent_id").ok(),
                "timestamp":        r.try_get::<DateTime<Utc>, _>("timestamp").ok().map(|t| t.to_rfc3339()),
                "latency_ms":       r.try_get::<Option<i32>, _>("latency_ms").ok().flatten(),
                "direction":        r.try_get::<String, _>("direction").ok(),
                "method":           r.try_get::<String, _>("method").ok(),
                "upstream_target":  r.try_get::<String, _>("upstream_target").ok(),
                "provider":         r.try_get::<String, _>("provider").ok(),
                "model":            r.try_get::<Option<String>, _>("model").ok().flatten(),
                "status_code":      r.try_get::<Option<i32>, _>("status_code").ok().flatten(),
                "finish_reason":    r.try_get::<Option<String>, _>("finish_reason").ok().flatten(),
                "raw_size_bytes":   r.try_get::<Option<i64>, _>("raw_size_bytes").ok().flatten(),
                "input_tokens":     r.try_get::<Option<i32>, _>("input_tokens").ok().flatten(),
                "output_tokens":    r.try_get::<Option<i32>, _>("output_tokens").ok().flatten(),
                "total_tokens":     r.try_get::<Option<i32>, _>("total_tokens").ok().flatten(),
                "cost_usd":         r.try_get::<Option<f64>, _>("cost_usd").ok().flatten(),
                "lineage_hash":     r.try_get::<String, _>("lineage_hash").ok(),
                "compliance_tag":   r.try_get::<String, _>("compliance_tag").ok(),
                "error_message":    r.try_get::<Option<String>, _>("error_message").ok().flatten(),
                "created_at":       r.try_get::<DateTime<Utc>, _>("created_at").ok().map(|t| t.to_rfc3339()),
                // Tracing columns
                "event_kind":       r.try_get::<Option<String>, _>("event_kind").ok().flatten(),
                "span_id":          r.try_get::<Option<Uuid>, _>("span_id").ok().flatten().map(|u| u.to_string()),
                "parent_span_id":   r.try_get::<Option<Uuid>, _>("parent_span_id").ok().flatten().map(|u| u.to_string()),
                "trace_id":         r.try_get::<Option<Uuid>, _>("trace_id").ok().flatten().map(|u| u.to_string()),
                "tool_name":        r.try_get::<Option<String>, _>("tool_name").ok().flatten(),
                "tool_args":        r.try_get::<Option<serde_json::Value>, _>("tool_args").ok().flatten(),
                "tool_result":      r.try_get::<Option<serde_json::Value>, _>("tool_result").ok().flatten(),
                "mcp_server":       r.try_get::<Option<String>, _>("mcp_server").ok().flatten(),
                "risk_score":       r.try_get::<Option<f32>, _>("risk_score").ok().flatten(),
                "external_trace_id":r.try_get::<Option<String>, _>("external_trace_id").ok().flatten(),
            })
        })
        .collect();

    Ok(result)
}

/// List recent traces, ordered by started_at descending.
pub async fn list_traces(
    pool: &StorePool,
    root_agent_id: Option<&str>,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {

    let mut conditions: Vec<String> = Vec::new();
    let mut param_idx = 1usize;

    if root_agent_id.is_some() {
        conditions.push(format!("root_agent_id = ${param_idx}"));
        param_idx += 1;
    }
    if status.is_some() {
        conditions.push(format!("status = ${param_idx}"));
        param_idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let limit_param = param_idx;
    let offset_param = param_idx + 1;

    let sql = format!(
        r#"
        SELECT
            trace_id, root_agent_id, task_description, status,
            stopped_by, error_message,
            started_at, completed_at, created_at,
            total_cost_usd, peak_risk_score, event_count, agent_count,
            external_trace_id
        FROM traces
        {where_clause}
        ORDER BY started_at DESC
        LIMIT ${limit_param} OFFSET ${offset_param}
        "#,
    );

    let mut q = sqlx::query(&sql);
    if let Some(v) = root_agent_id {
        q = q.bind(v);
    }
    if let Some(v) = status {
        q = q.bind(v);
    }
    q = q.bind(limit).bind(offset);

    let rows = q.fetch_all(pool).await?;
    Ok(rows.iter().map(|r| row_to_json(r)).collect())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_json(r: &sqlx::postgres::PgRow) -> serde_json::Value {
    serde_json::json!({
        "trace_id":          r.try_get::<Uuid, _>("trace_id").ok().map(|u| u.to_string()),
        "root_agent_id":     r.try_get::<String, _>("root_agent_id").ok(),
        "task_description":  r.try_get::<Option<String>, _>("task_description").ok().flatten(),
        "status":            r.try_get::<String, _>("status").ok(),
        "stopped_by":        r.try_get::<Option<String>, _>("stopped_by").ok().flatten(),
        "error_message":     r.try_get::<Option<String>, _>("error_message").ok().flatten(),
        "started_at":        r.try_get::<DateTime<Utc>, _>("started_at").ok().map(|t| t.to_rfc3339()),
        "completed_at":      r.try_get::<Option<DateTime<Utc>>, _>("completed_at").ok().flatten().map(|t| t.to_rfc3339()),
        "created_at":        r.try_get::<DateTime<Utc>, _>("created_at").ok().map(|t| t.to_rfc3339()),
        "total_cost_usd":    r.try_get::<Option<f64>, _>("total_cost_usd").ok().flatten(),
        "peak_risk_score":   r.try_get::<Option<f32>, _>("peak_risk_score").ok().flatten(),
        "event_count":       r.try_get::<Option<i32>, _>("event_count").ok().flatten(),
        "agent_count":       r.try_get::<Option<i32>, _>("agent_count").ok().flatten(),
        "external_trace_id": r.try_get::<Option<String>, _>("external_trace_id").ok().flatten(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use govrix_scout_common::models::trace::TraceStatus;

    #[test]
    fn trace_new_is_running() {
        let t = Trace::new("agent-001");
        assert_eq!(t.status, TraceStatus::Running);
        assert_eq!(t.agent_count, 1);
    }

    #[test]
    fn trace_status_display() {
        assert_eq!(TraceStatus::Running.to_string(), "running");
        assert_eq!(TraceStatus::Completed.to_string(), "completed");
        assert_eq!(TraceStatus::Stopped.to_string(), "stopped");
        assert_eq!(TraceStatus::Failed.to_string(), "failed");
    }
}
