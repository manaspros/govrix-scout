//! Persistent session CRUD — maps to the `sessions` table (migration 010).
//!
//! Sessions provide durable tracking across proxy restarts. The in-memory
//! session map is rebuilt from active rows at startup.
//!
//! All writes are fire-and-forget compatible — callers should `tokio::spawn`
//! these functions to keep the hot path latency-free.

use chrono::{DateTime, Utc};
use govrix_scout_common::models::session::Session;
#[allow(unused_imports)]
use govrix_scout_common::models::session::SessionStatus;
use sqlx::Row;
use uuid::Uuid;

use crate::db::StorePool;

// ── Write path ────────────────────────────────────────────────────────────────

/// Upsert a session record.
///
/// Creates the row if it doesn't exist, updates lifecycle fields if it does.
/// Idempotent — safe to call on every event.
pub async fn upsert_session(pool: &StorePool, session: &Session) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO sessions (
            session_id, agent_id, trace_id, status,
            started_at, last_event_at, created_at,
            event_count, total_cost_usd,
            killed_at, killed_by, kill_reason
        ) VALUES (
            $1, $2, $3, $4,
            $5, $6, $7,
            $8, $9,
            $10, $11, $12
        )
        ON CONFLICT (session_id) DO UPDATE SET
            trace_id       = EXCLUDED.trace_id,
            status         = EXCLUDED.status,
            last_event_at  = EXCLUDED.last_event_at,
            event_count    = EXCLUDED.event_count,
            total_cost_usd = EXCLUDED.total_cost_usd,
            killed_at      = EXCLUDED.killed_at,
            killed_by      = EXCLUDED.killed_by,
            kill_reason    = EXCLUDED.kill_reason
        "#,
    )
    .bind(&session.session_id)
    .bind(&session.agent_id)
    .bind(session.trace_id)
    .bind(session.status.to_string())
    .bind(session.started_at)
    .bind(session.last_event_at)
    .bind(session.created_at)
    .bind(session.event_count)
    .bind(session.total_cost_usd)
    .bind(session.killed_at)
    .bind(&session.killed_by)
    .bind(&session.kill_reason)
    .execute(pool)
    .await?;

    Ok(())
}

/// Increment event counter and cost for a session.
///
/// Atomic UPDATE — does not require reading the full session first.
pub async fn update_session_stats(
    pool: &StorePool,
    session_id: &str,
    event_count_delta: i64,
    cost_delta: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE sessions
        SET
            event_count    = event_count + $2,
            total_cost_usd = total_cost_usd + $3,
            last_event_at  = NOW()
        WHERE session_id = $1
        "#,
    )
    .bind(session_id)
    .bind(event_count_delta)
    .bind(cost_delta)
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a session as killed.
///
/// Sets status='killed', killed_at=NOW(), and records who/why.
/// Safe to call concurrently — single atomic UPDATE.
pub async fn kill_session(
    pool: &StorePool,
    session_id: &str,
    killed_by: &str,
    reason: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE sessions
        SET
            status      = 'killed',
            killed_at   = NOW(),
            killed_by   = $2,
            kill_reason = $3
        WHERE session_id = $1
        "#,
    )
    .bind(session_id)
    .bind(killed_by)
    .bind(reason)
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a session as completed (idle timeout or natural end).
///
/// Does not set killed_at — use `kill_session` for forced terminations.
pub async fn complete_session(
    pool: &StorePool,
    session_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE sessions
        SET status = 'completed'
        WHERE session_id = $1
          AND status NOT IN ('killed', 'completed')
        "#,
    )
    .bind(session_id)
    .execute(pool)
    .await?;

    Ok(())
}

// ── Read path ─────────────────────────────────────────────────────────────────

/// Get a session by its ID.
///
/// Returns the raw JSONB representation.
pub async fn get_session(
    pool: &StorePool,
    session_id: &str,
) -> Result<Option<serde_json::Value>, sqlx::Error> {

    let row = sqlx::query(
        r#"
        SELECT
            session_id, agent_id, trace_id, status,
            started_at, last_event_at, created_at,
            event_count, total_cost_usd,
            killed_at, killed_by, kill_reason
        FROM sessions
        WHERE session_id = $1
        LIMIT 1
        "#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| row_to_json(&r)))
}

/// List sessions for a given agent, ordered by last_event_at descending.
pub async fn list_sessions_for_agent(
    pool: &StorePool,
    agent_id: &str,
    limit: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {

    let rows = sqlx::query(
        r#"
        SELECT
            session_id, agent_id, trace_id, status,
            started_at, last_event_at, created_at,
            event_count, total_cost_usd,
            killed_at, killed_by, kill_reason
        FROM sessions
        WHERE agent_id = $1
        ORDER BY last_event_at DESC
        LIMIT $2
        "#,
    )
    .bind(agent_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|r| row_to_json(r)).collect())
}

/// List all active sessions, ordered by last_event_at descending.
///
/// Used at startup to rebuild the in-memory session cache.
pub async fn list_active_sessions(
    pool: &StorePool,
    limit: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {

    let rows = sqlx::query(
        r#"
        SELECT
            session_id, agent_id, trace_id, status,
            started_at, last_event_at, created_at,
            event_count, total_cost_usd,
            killed_at, killed_by, kill_reason
        FROM sessions
        WHERE status = 'active'
        ORDER BY last_event_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|r| row_to_json(r)).collect())
}

/// List sessions with optional filters.
pub async fn list_sessions(
    pool: &StorePool,
    agent_id: Option<&str>,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {

    let mut conditions: Vec<String> = Vec::new();
    let mut param_idx = 1usize;

    if agent_id.is_some() {
        conditions.push(format!("agent_id = ${param_idx}"));
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
            session_id, agent_id, trace_id, status,
            started_at, last_event_at, created_at,
            event_count, total_cost_usd,
            killed_at, killed_by, kill_reason
        FROM sessions
        {where_clause}
        ORDER BY last_event_at DESC
        LIMIT ${limit_param} OFFSET ${offset_param}
        "#,
    );

    let mut q = sqlx::query(&sql);
    if let Some(v) = agent_id {
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
        "session_id":     r.try_get::<String, _>("session_id").ok(),
        "agent_id":       r.try_get::<String, _>("agent_id").ok(),
        "trace_id":       r.try_get::<Option<Uuid>, _>("trace_id").ok().flatten().map(|u| u.to_string()),
        "status":         r.try_get::<String, _>("status").ok(),
        "started_at":     r.try_get::<DateTime<Utc>, _>("started_at").ok().map(|t| t.to_rfc3339()),
        "last_event_at":  r.try_get::<DateTime<Utc>, _>("last_event_at").ok().map(|t| t.to_rfc3339()),
        "created_at":     r.try_get::<DateTime<Utc>, _>("created_at").ok().map(|t| t.to_rfc3339()),
        "event_count":    r.try_get::<Option<i32>, _>("event_count").ok().flatten(),
        "total_cost_usd": r.try_get::<Option<f64>, _>("total_cost_usd").ok().flatten(),
        "killed_at":      r.try_get::<Option<DateTime<Utc>>, _>("killed_at").ok().flatten().map(|t| t.to_rfc3339()),
        "killed_by":      r.try_get::<Option<String>, _>("killed_by").ok().flatten(),
        "kill_reason":    r.try_get::<Option<String>, _>("kill_reason").ok().flatten(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_status_round_trips_as_string() {
        assert_eq!(SessionStatus::Active.to_string(), "active");
        assert_eq!(SessionStatus::Killed.to_string(), "killed");
        assert_eq!(SessionStatus::Completed.to_string(), "completed");
        assert_eq!(SessionStatus::Idle.to_string(), "idle");
    }

    #[test]
    fn session_new_has_zero_stats() {
        let s = Session::new("sess-test", "agent-test");
        assert_eq!(s.event_count, 0);
        assert_eq!(s.total_cost_usd, 0.0);
        assert!(s.trace_id.is_none());
    }
}
