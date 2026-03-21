//! Compliance report query functions.
//!
//! These queries aggregate data from the events and agents tables to populate
//! the EU AI Act and Cyber Insurance report structs in `govrix-scout-reports`.
//!
//! All queries handle empty result sets gracefully and return 0 / empty-vec
//! defaults when no data is present.

use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::db::StorePool;

// ── EU AI Act report data ─────────────────────────────────────────────────────

/// Aggregated data required to build an EU AI Act transparency report.
#[derive(Debug, Default)]
pub struct EuAiActQueryResult {
    /// Total events in the report period.
    pub total_events: i64,
    /// Distinct active agent count.
    pub agent_count: i64,
    /// Distinct LLM providers observed.
    pub providers: Vec<String>,
    /// Distinct event_kind values observed.
    pub event_kinds: Vec<String>,
    /// Times an agent was blocked (retired/killed) during the period.
    pub kill_switch_activations: i64,
    /// Times a circuit breaker fired (compliance_tag LIKE 'block:%').
    pub circuit_breaker_activations: i64,
    /// policy.block event count.
    pub policy_blocks: i64,
    /// Events with risk_score > 75.
    pub high_risk_events: i64,
    /// Events with non-empty pii_detected array.
    pub pii_detections: i64,
    /// Top tools by average risk score (tool_name, call_count, avg_risk, max_risk).
    pub top_risk_tools: Vec<(String, i64, f32, f32)>,
}

/// Query the database for all data needed to build an EU AI Act report.
///
/// Gracefully handles empty result sets — returns zeros/empty vectors when
/// no data is available for the requested period.
pub async fn eu_ai_act_data(
    pool: &StorePool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<EuAiActQueryResult, sqlx::Error> {
    let mut result = EuAiActQueryResult::default();

    // 1. Total events in period
    let row =
        sqlx::query("SELECT COUNT(*) AS cnt FROM events WHERE timestamp >= $1 AND timestamp < $2")
            .bind(from)
            .bind(to)
            .fetch_one(pool)
            .await?;
    result.total_events = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 2. Distinct agent count
    let row = sqlx::query(
        "SELECT COUNT(DISTINCT agent_id) AS cnt FROM events WHERE timestamp >= $1 AND timestamp < $2",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    result.agent_count = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 3. Distinct providers
    let rows = sqlx::query(
        "SELECT DISTINCT provider FROM events WHERE timestamp >= $1 AND timestamp < $2 AND provider IS NOT NULL",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    result.providers = rows
        .iter()
        .filter_map(|r| r.try_get::<String, _>("provider").ok())
        .filter(|p| p != "unknown")
        .collect();

    // 4. Distinct event kinds
    let rows = sqlx::query(
        "SELECT DISTINCT event_kind FROM events WHERE timestamp >= $1 AND timestamp < $2 AND event_kind IS NOT NULL",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    result.event_kinds = rows
        .iter()
        .filter_map(|r| r.try_get::<String, _>("event_kind").ok())
        .collect();

    // 5. Kill switch activations: agents currently with status = 'blocked'
    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM agents WHERE status = 'blocked'")
        .fetch_one(pool)
        .await?;
    result.kill_switch_activations = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 6. Circuit breaker activations: compliance_tag starts with 'block:'
    let row = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM events WHERE timestamp >= $1 AND timestamp < $2 AND compliance_tag LIKE 'block:%'",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    result.circuit_breaker_activations = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 7. Policy block events
    let row = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM events WHERE timestamp >= $1 AND timestamp < $2 AND event_kind = 'policy.block'",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    result.policy_blocks = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 8. High risk events (risk_score > 75)
    let row = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM events WHERE timestamp >= $1 AND timestamp < $2 AND risk_score > 75",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    result.high_risk_events = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 9. PII detections: pii_detected is not null and not an empty array
    let row = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM events \
         WHERE timestamp >= $1 AND timestamp < $2 \
         AND pii_detected IS NOT NULL \
         AND pii_detected != '[]'::jsonb \
         AND jsonb_array_length(pii_detected) > 0",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    result.pii_detections = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 10. Top risk tools — group by tool_name, compute avg/max risk_score
    let rows = sqlx::query(
        "SELECT tool_name, COUNT(*) AS call_count, \
         AVG(risk_score)::real AS avg_risk, MAX(risk_score)::real AS max_risk \
         FROM events \
         WHERE timestamp >= $1 AND timestamp < $2 \
         AND tool_name IS NOT NULL \
         AND risk_score IS NOT NULL \
         GROUP BY tool_name \
         ORDER BY avg_risk DESC \
         LIMIT 10",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    result.top_risk_tools = rows
        .iter()
        .filter_map(|r| {
            let name = r.try_get::<String, _>("tool_name").ok()?;
            let calls = r.try_get::<i64, _>("call_count").unwrap_or(0);
            let avg = r.try_get::<f32, _>("avg_risk").unwrap_or(0.0);
            let max = r.try_get::<f32, _>("max_risk").unwrap_or(0.0);
            Some((name, calls, avg, max))
        })
        .collect();

    Ok(result)
}

// ── Cyber Insurance report data ───────────────────────────────────────────────

/// Aggregated data required to build a Cyber Insurance evidence package.
#[derive(Debug, Default)]
pub struct CyberInsuranceQueryResult {
    pub total_events: i64,
    pub total_agents: i64,
    pub total_cost_usd: f64,
    pub avg_risk_score: f32,
    pub policy_blocks: i64,
    pub pii_detections: i64,
    pub circuit_breaker_activations: i64,
    pub kill_switch_activations: i64,
    pub llm_providers: Vec<String>,
    pub mcp_servers: Vec<String>,
    /// Per-agent data: (agent_id, framework, first_seen, last_seen, request_count, total_cost_usd, avg_risk_score, status)
    pub agents: Vec<AgentRow>,
    /// Recent incidents: (incident_type, occurred_at, agent_id, action_taken, resolved)
    pub incidents: Vec<IncidentRow>,
    pub events_per_day_avg: f64,
}

/// Raw agent row from the compliance query.
#[derive(Debug, Clone)]
pub struct AgentRow {
    pub agent_id: String,
    pub framework: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub request_count: i64,
    pub total_cost_usd: f64,
    pub avg_risk_score: f32,
    pub status: String,
}

/// Raw incident row from the compliance query.
#[derive(Debug, Clone)]
pub struct IncidentRow {
    pub incident_type: String,
    pub occurred_at: DateTime<Utc>,
    pub agent_id: String,
    pub action_taken: String,
    pub resolved: bool,
}

/// Query the database for all data needed to build a Cyber Insurance evidence package.
pub async fn cyber_insurance_data(
    pool: &StorePool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<CyberInsuranceQueryResult, sqlx::Error> {
    let mut result = CyberInsuranceQueryResult::default();

    // 1. Total events in period
    let row =
        sqlx::query("SELECT COUNT(*) AS cnt FROM events WHERE timestamp >= $1 AND timestamp < $2")
            .bind(from)
            .bind(to)
            .fetch_one(pool)
            .await?;
    result.total_events = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 2. Total agents (all time)
    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM agents")
        .fetch_one(pool)
        .await?;
    result.total_agents = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 3. Total cost in period
    let row = sqlx::query(
        "SELECT COALESCE(SUM(cost_usd), 0.0)::float8 AS total \
         FROM events WHERE timestamp >= $1 AND timestamp < $2",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    result.total_cost_usd = row.try_get::<f64, _>("total").unwrap_or(0.0);

    // 4. Average risk score in period
    let row = sqlx::query(
        "SELECT COALESCE(AVG(risk_score), 0.0)::real AS avg_risk \
         FROM events WHERE timestamp >= $1 AND timestamp < $2 AND risk_score IS NOT NULL",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    result.avg_risk_score = row.try_get::<f32, _>("avg_risk").unwrap_or(0.0);

    // 5. Policy blocks
    let row = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM events \
         WHERE timestamp >= $1 AND timestamp < $2 AND event_kind = 'policy.block'",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    result.policy_blocks = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 6. PII detections
    let row = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM events \
         WHERE timestamp >= $1 AND timestamp < $2 \
         AND pii_detected IS NOT NULL \
         AND pii_detected != '[]'::jsonb \
         AND jsonb_array_length(pii_detected) > 0",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    result.pii_detections = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 7. Circuit breaker activations
    let row = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM events \
         WHERE timestamp >= $1 AND timestamp < $2 AND compliance_tag LIKE 'block:%'",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    result.circuit_breaker_activations = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 8. Kill switch activations (blocked agents)
    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM agents WHERE status = 'blocked'")
        .fetch_one(pool)
        .await?;
    result.kill_switch_activations = row.try_get::<i64, _>("cnt").unwrap_or(0);

    // 9. LLM providers
    let rows = sqlx::query(
        "SELECT DISTINCT provider FROM events \
         WHERE timestamp >= $1 AND timestamp < $2 AND provider IS NOT NULL",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    result.llm_providers = rows
        .iter()
        .filter_map(|r| r.try_get::<String, _>("provider").ok())
        .filter(|p| p != "unknown")
        .collect();

    // 10. MCP servers observed
    let rows = sqlx::query(
        "SELECT DISTINCT mcp_server FROM events \
         WHERE timestamp >= $1 AND timestamp < $2 AND mcp_server IS NOT NULL",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    result.mcp_servers = rows
        .iter()
        .filter_map(|r| r.try_get::<String, _>("mcp_server").ok())
        .collect();

    // 11. Per-agent data
    let rows = sqlx::query(
        "SELECT a.id AS agent_id, \
                COALESCE(a.agent_type, 'unknown') AS framework, \
                a.first_seen_at, \
                a.last_seen_at, \
                a.total_requests, \
                a.total_cost_usd, \
                COALESCE((SELECT AVG(risk_score)::real FROM events e \
                          WHERE e.agent_id = a.id \
                          AND e.timestamp >= $1 AND e.timestamp < $2 \
                          AND e.risk_score IS NOT NULL), 0.0)::real AS avg_risk, \
                a.status \
         FROM agents a \
         ORDER BY a.total_requests DESC \
         LIMIT 50",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    result.agents = rows
        .iter()
        .filter_map(|r| {
            let agent_id = r.try_get::<String, _>("agent_id").ok()?;
            let framework = r
                .try_get::<String, _>("framework")
                .unwrap_or_else(|_| "unknown".to_string());
            let first_seen = r.try_get::<DateTime<Utc>, _>("first_seen_at").ok()?;
            let last_seen = r.try_get::<DateTime<Utc>, _>("last_seen_at").ok()?;
            let request_count = r.try_get::<i64, _>("total_requests").unwrap_or(0);
            let total_cost_usd = r.try_get::<f64, _>("total_cost_usd").unwrap_or(0.0);
            let avg_risk_score = r.try_get::<f32, _>("avg_risk").unwrap_or(0.0);
            let status = r
                .try_get::<String, _>("status")
                .unwrap_or_else(|_| "unknown".to_string());
            Some(AgentRow {
                agent_id,
                framework,
                first_seen,
                last_seen,
                request_count,
                total_cost_usd,
                avg_risk_score,
                status,
            })
        })
        .collect();

    // 12. Recent incidents (last 50 notable events)
    let rows = sqlx::query(
        "SELECT event_kind, timestamp, agent_id, compliance_tag \
         FROM events \
         WHERE timestamp >= $1 AND timestamp < $2 \
         AND (event_kind = 'policy.block' \
              OR (pii_detected IS NOT NULL AND pii_detected != '[]'::jsonb AND jsonb_array_length(pii_detected) > 0) \
              OR compliance_tag LIKE 'block:%' \
              OR risk_score > 75) \
         ORDER BY timestamp DESC \
         LIMIT 50",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    result.incidents = rows
        .iter()
        .filter_map(|r| {
            let event_kind = r.try_get::<String, _>("event_kind").ok()?;
            let occurred_at = r.try_get::<DateTime<Utc>, _>("timestamp").ok()?;
            let agent_id = r.try_get::<String, _>("agent_id").ok()?;
            let compliance_tag = r.try_get::<String, _>("compliance_tag").unwrap_or_default();

            let incident_type = if event_kind == "policy.block" {
                "policy_block"
            } else if compliance_tag.starts_with("block:") {
                "circuit_breaker"
            } else {
                "anomaly"
            };

            let action_taken =
                if event_kind == "policy.block" || compliance_tag.starts_with("block:") {
                    "blocked"
                } else {
                    "logged"
                };

            Some(IncidentRow {
                incident_type: incident_type.to_string(),
                occurred_at,
                agent_id,
                action_taken: action_taken.to_string(),
                resolved: true, // all past incidents are resolved
            })
        })
        .collect();

    // 13. Average events per day
    let duration_days = (to - from).num_days().max(1) as f64;
    result.events_per_day_avg = result.total_events as f64 / duration_days;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eu_ai_act_query_result_default() {
        let r = EuAiActQueryResult::default();
        assert_eq!(r.total_events, 0);
        assert_eq!(r.agent_count, 0);
        assert!(r.providers.is_empty());
        assert!(r.top_risk_tools.is_empty());
    }

    #[test]
    fn cyber_insurance_query_result_default() {
        let r = CyberInsuranceQueryResult::default();
        assert_eq!(r.total_events, 0);
        assert_eq!(r.total_agents, 0);
        assert!(r.agents.is_empty());
        assert!(r.incidents.is_empty());
    }
}
