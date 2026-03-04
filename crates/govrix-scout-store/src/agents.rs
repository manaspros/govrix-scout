//! Agent registry — upsert and query the `agents` table.
//!
//! The agents table tracks identity, statistics, and policy assignments
//! for every AI agent observed by the proxy.

use chrono::Utc;
use govrix_scout_common::models::agent::Agent;

use crate::db::StorePool;

// ── Filter type ───────────────────────────────────────────────────────────────

/// Optional filter parameters for listing agents.
#[derive(Debug, Clone, Default)]
pub struct AgentFilter {
    /// Filter by lifecycle status (active / idle / error / blocked).
    pub status: Option<String>,
    /// Filter by agent type (langchain / mcp_client / …).
    pub agent_type: Option<String>,
    /// Full-text substring match on `name`.
    pub name_contains: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

impl AgentFilter {
    pub fn new() -> Self {
        Self {
            limit: 100,
            ..Default::default()
        }
    }
}

// ── Write path ────────────────────────────────────────────────────────────────

/// Upsert an agent record.
///
/// On conflict (same `id`), updates `last_seen_at`, `status`, `total_requests`,
/// `total_tokens_in`, `total_tokens_out`, `total_cost_usd`, and `last_model_used`.
pub async fn upsert_agent(pool: &StorePool, agent: &Agent) -> Result<(), sqlx::Error> {
    let agent_type = agent.agent_type.to_string();
    let status = agent.status.to_string();
    let cost_str = agent.total_cost_usd.to_string();
    let cost_f64: f64 = cost_str.parse().unwrap_or(0.0);

    sqlx::query(
        r#"
        INSERT INTO agents (
            id, name, description, agent_type, status,
            first_seen_at, last_seen_at,
            source_ip, fingerprint,
            target_apis, mcp_servers,
            total_requests, total_tokens_in, total_tokens_out, total_cost_usd,
            last_model_used, error_count,
            labels, metadata,
            created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5,
            $6, $7,
            $8::inet, $9,
            $10, $11,
            $12, $13, $14, $15,
            $16, $17,
            $18, $19,
            $20, $21
        )
        ON CONFLICT (id) DO UPDATE SET
            last_seen_at     = EXCLUDED.last_seen_at,
            status           = EXCLUDED.status,
            total_requests   = EXCLUDED.total_requests,
            total_tokens_in  = EXCLUDED.total_tokens_in,
            total_tokens_out = EXCLUDED.total_tokens_out,
            total_cost_usd   = EXCLUDED.total_cost_usd,
            last_model_used  = EXCLUDED.last_model_used,
            error_count      = EXCLUDED.error_count,
            updated_at       = now()
        "#,
    )
    .bind(&agent.id)
    .bind(&agent.name)
    .bind(&agent.description)
    .bind(&agent_type)
    .bind(&status)
    .bind(agent.first_seen_at)
    .bind(agent.last_seen_at)
    .bind(&agent.source_ip)
    .bind(&agent.fingerprint)
    .bind(&agent.target_apis)
    .bind(&agent.mcp_servers)
    .bind(agent.total_requests)
    .bind(agent.total_tokens_in)
    .bind(agent.total_tokens_out)
    .bind(cost_f64)
    .bind(&agent.last_model_used)
    .bind(agent.error_count)
    .bind(&agent.labels)
    .bind(&agent.metadata)
    .bind(agent.created_at)
    .bind(agent.updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

// ── Read path ─────────────────────────────────────────────────────────────────

/// Get a single agent by its VARCHAR primary key.
pub async fn get_agent(
    pool: &StorePool,
    id: &str,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT
            id, name, description, agent_type, status,
            first_seen_at, last_seen_at, last_error_at,
            source_ip, fingerprint,
            target_apis, mcp_servers,
            total_requests, total_tokens_in, total_tokens_out, total_cost_usd,
            last_model_used, error_count,
            labels, metadata,
            created_at, updated_at
        FROM agents
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| build_agent_json(&r)))
}

/// List agents with optional filters, ordered by `last_seen_at` DESC.
pub async fn list_agents(
    pool: &StorePool,
    filter: &AgentFilter,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let mut conditions: Vec<String> = Vec::new();
    let mut param_idx = 1usize;

    if filter.status.is_some() {
        conditions.push(format!("status = ${param_idx}"));
        param_idx += 1;
    }
    if filter.agent_type.is_some() {
        conditions.push(format!("agent_type = ${param_idx}"));
        param_idx += 1;
    }
    if filter.name_contains.is_some() {
        conditions.push(format!("name ILIKE ${param_idx}"));
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
            id, name, description, agent_type, status,
            first_seen_at, last_seen_at, last_error_at,
            source_ip, fingerprint,
            target_apis, mcp_servers,
            total_requests, total_tokens_in, total_tokens_out, total_cost_usd,
            last_model_used, error_count,
            labels, metadata,
            created_at, updated_at
        FROM agents
        {where_clause}
        ORDER BY last_seen_at DESC
        LIMIT ${limit_param} OFFSET ${offset_param}
        "#,
        where_clause = where_clause,
        limit_param = limit_param,
        offset_param = offset_param,
    );

    let mut q = sqlx::query(&sql);

    if let Some(ref v) = filter.status {
        q = q.bind(v);
    }
    if let Some(ref v) = filter.agent_type {
        q = q.bind(v);
    }
    if let Some(ref v) = filter.name_contains {
        // Wrap in % for ILIKE
        q = q.bind(format!("%{}%", v));
    }
    q = q.bind(filter.limit).bind(filter.offset);

    let rows = q.fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| build_agent_json(&r)).collect())
}

/// Update an agent's `last_seen_at` timestamp atomically.
///
/// Called on every proxied request as a lightweight "heartbeat" write.
pub async fn touch_agent(pool: &StorePool, agent_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE agents
        SET last_seen_at = now(), updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(agent_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Retire an agent by setting its status to 'blocked'.
///
/// A retired agent can no longer initiate new proxy sessions.
/// Existing sessions continue until they naturally expire.
pub async fn retire_agent(pool: &StorePool, agent_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE agents
        SET status = 'blocked', updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(agent_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Increment request/token/cost counters for an agent atomically.
///
/// Called by the background batch writer after each event is persisted.
/// Uses a single UPDATE to avoid read-modify-write races.
pub async fn increment_agent_stats(
    pool: &StorePool,
    agent_id: &str,
    tokens_in: i64,
    tokens_out: i64,
    cost_usd: f64,
    model: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE agents
        SET
            total_requests   = total_requests + 1,
            total_tokens_in  = total_tokens_in + $2,
            total_tokens_out = total_tokens_out + $3,
            total_cost_usd   = total_cost_usd + $4,
            last_model_used  = COALESCE($5, last_model_used),
            last_seen_at     = now(),
            updated_at       = now()
        WHERE id = $1
        "#,
    )
    .bind(agent_id)
    .bind(tokens_in)
    .bind(tokens_out)
    .bind(cost_usd)
    .bind(model)
    .execute(pool)
    .await?;

    Ok(())
}

/// Record an error for an agent: increment error_count and set last_error_at.
pub async fn record_agent_error(pool: &StorePool, agent_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE agents
        SET
            error_count  = error_count + 1,
            last_error_at = now(),
            status       = CASE WHEN status = 'active' THEN 'error' ELSE status END,
            updated_at   = now()
        WHERE id = $1
        "#,
    )
    .bind(agent_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Return the count of active agents (OSS 25-agent soft limit check).
pub async fn active_agent_count(pool: &StorePool) -> Result<i64, sqlx::Error> {
    use sqlx::Row;

    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM agents WHERE status != 'blocked'")
        .fetch_one(pool)
        .await?;

    Ok(row.try_get::<i64, _>("cnt").unwrap_or(0))
}

/// Update mutable agent metadata fields (name, description, labels).
///
/// Only updates fields where `Some(value)` is provided. Uses a dynamic
/// SET clause — only touches columns explicitly supplied.
pub async fn update_agent_metadata(
    pool: &StorePool,
    agent_id: &str,
    name: Option<&str>,
    description: Option<&str>,
    labels: Option<&serde_json::Value>,
) -> Result<(), sqlx::Error> {
    let mut set_parts: Vec<String> = Vec::new();
    let mut param_idx = 1usize;

    if name.is_some() {
        set_parts.push(format!("name = ${param_idx}"));
        param_idx += 1;
    }
    if description.is_some() {
        set_parts.push(format!("description = ${param_idx}"));
        param_idx += 1;
    }
    if labels.is_some() {
        set_parts.push(format!("labels = ${param_idx}"));
        param_idx += 1;
    }
    set_parts.push("updated_at = now()".to_string());

    let sql = format!(
        "UPDATE agents SET {} WHERE id = ${}",
        set_parts.join(", "),
        param_idx
    );

    let mut q = sqlx::query(&sql);
    if let Some(v) = name {
        q = q.bind(v);
    }
    if let Some(v) = description {
        q = q.bind(v);
    }
    if let Some(v) = labels {
        q = q.bind(v);
    }
    q = q.bind(agent_id);
    q.execute(pool).await?;
    Ok(())
}

/// Get legacy `get_agent_by_id` alias for backwards compatibility.
pub async fn get_agent_by_id(
    pool: &StorePool,
    id: &str,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    get_agent(pool, id).await
}

/// Legacy `list_agents` with simple limit/offset (no filter struct).
pub async fn list_agents_simple(
    pool: &StorePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let filter = AgentFilter {
        limit,
        offset,
        ..Default::default()
    };
    list_agents(pool, &filter).await
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn build_agent_json(r: &sqlx::postgres::PgRow) -> serde_json::Value {
    use chrono::DateTime;
    use sqlx::Row;

    serde_json::json!({
        "id":               r.try_get::<String, _>("id").ok(),
        "name":             r.try_get::<Option<String>, _>("name").ok().flatten(),
        "description":      r.try_get::<Option<String>, _>("description").ok().flatten(),
        "agent_type":       r.try_get::<String, _>("agent_type").ok(),
        "status":           r.try_get::<String, _>("status").ok(),
        "first_seen_at":    r.try_get::<DateTime<Utc>, _>("first_seen_at").ok().map(|t| t.to_rfc3339()),
        "last_seen_at":     r.try_get::<DateTime<Utc>, _>("last_seen_at").ok().map(|t| t.to_rfc3339()),
        "last_error_at":    r.try_get::<Option<DateTime<Utc>>, _>("last_error_at").ok().flatten().map(|t| t.to_rfc3339()),
        "source_ip":        r.try_get::<Option<String>, _>("source_ip").ok().flatten(),
        "fingerprint":      r.try_get::<Option<String>, _>("fingerprint").ok().flatten(),
        "target_apis":      r.try_get::<Option<serde_json::Value>, _>("target_apis").ok().flatten(),
        "mcp_servers":      r.try_get::<Option<serde_json::Value>, _>("mcp_servers").ok().flatten(),
        "total_requests":   r.try_get::<i64, _>("total_requests").ok(),
        "total_tokens_in":  r.try_get::<i64, _>("total_tokens_in").ok(),
        "total_tokens_out": r.try_get::<i64, _>("total_tokens_out").ok(),
        "total_cost_usd":   r.try_get::<f64, _>("total_cost_usd").ok(),
        "last_model_used":  r.try_get::<Option<String>, _>("last_model_used").ok().flatten(),
        "error_count":      r.try_get::<i64, _>("error_count").ok(),
        "labels":           r.try_get::<Option<serde_json::Value>, _>("labels").ok().flatten(),
        "metadata":         r.try_get::<Option<serde_json::Value>, _>("metadata").ok().flatten(),
        "created_at":       r.try_get::<DateTime<Utc>, _>("created_at").ok().map(|t| t.to_rfc3339()),
        "updated_at":       r.try_get::<DateTime<Utc>, _>("updated_at").ok().map(|t| t.to_rfc3339()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_filter_default_limit() {
        let f = AgentFilter::new();
        assert_eq!(f.limit, 100);
        assert_eq!(f.offset, 0);
        assert!(f.status.is_none());
        assert!(f.agent_type.is_none());
    }

    #[test]
    fn agent_filter_sql_building() {
        let filter = AgentFilter {
            status: Some("active".to_string()),
            agent_type: Some("langchain".to_string()),
            limit: 25,
            offset: 0,
            ..Default::default()
        };

        let mut conditions: Vec<String> = Vec::new();
        let mut param_idx = 1usize;

        if filter.status.is_some() {
            conditions.push(format!("status = ${param_idx}"));
            param_idx += 1;
        }
        if filter.agent_type.is_some() {
            conditions.push(format!("agent_type = ${param_idx}"));
            param_idx += 1;
        }

        assert_eq!(conditions.len(), 2);
        assert_eq!(param_idx, 3);
        assert_eq!(conditions[0], "status = $1");
        assert_eq!(conditions[1], "agent_type = $2");
    }

    #[test]
    fn name_contains_wraps_in_percent() {
        let name_contains = "gpt".to_string();
        let bound = format!("%{}%", name_contains);
        assert_eq!(bound, "%gpt%");
    }
}
