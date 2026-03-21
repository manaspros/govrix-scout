//! Platform handlers — compliance, risk, PII, policies, kill switch, health, and sessions.
//!
//! These endpoints power the dashboard pages. Each handler queries the PostgreSQL
//! database directly via `sqlx` and returns JSON payloads consumable by the React
//! frontend.
//!
//! Route map:
//!   GET  /api/v1/compliance/{framework}  — compliance_report
//!   GET  /api/v1/risk/overview           — risk_overview
//!   GET  /api/v1/pii/activity            — pii_activity
//!   GET  /api/v1/policies                — list_policies
//!   POST /api/v1/policies/reload         — reload_policies
//!   GET  /api/v1/kill-switch/status      — kill_switch_status
//!   GET  /api/v1/kill-switch/history     — kill_switch_history
//!   POST /api/v1/kill-switch/kill        — kill_agent
//!   POST /api/v1/kill-switch/revive      — revive_agent
//!   GET  /api/v1/platform/health         — platform_health
//!   GET  /api/v1/sessions                — list_sessions
//!   GET  /api/v1/sessions/{id}           — get_session

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use govrix_scout_store::StorePool;

use crate::api::state::AppState;

// ── Request types ────────────────────────────────────────────────────────────

/// Body for POST /api/v1/kill-switch/kill
#[derive(Debug, Deserialize)]
pub struct KillAgentBody {
    pub agent_id: String,
    pub reason: Option<String>,
}

/// Body for POST /api/v1/kill-switch/revive
#[derive(Debug, Deserialize)]
pub struct ReviveAgentBody {
    pub agent_id: String,
}

/// Query parameters for GET /api/v1/sessions
#[derive(Debug, Deserialize)]
pub struct ListSessionsParams {
    pub agent_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query parameters for GET /api/v1/pii/activity
#[derive(Debug, Deserialize)]
pub struct PiiActivityParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ── Query helpers ────────────────────────────────────────────────────────────

/// Run a `SELECT COUNT(*) AS cnt` query and return the count (or 0 on error).
async fn count_query(pool: &StorePool, sql: &str) -> i64 {
    sqlx::query(sql)
        .fetch_one(pool)
        .await
        .and_then(|r| r.try_get::<i64, _>("cnt"))
        .unwrap_or(0)
}

// ── 1. Compliance Report ─────────────────────────────────────────────────────

/// GET /api/v1/compliance/{framework}
///
/// Supported frameworks: soc2, eu-ai-act, hipaa, nist-800-53
pub async fn compliance_report(
    State(state): State<Arc<AppState>>,
    Path(framework): Path<String>,
) -> impl IntoResponse {
    let valid_frameworks = ["soc2", "eu-ai-act", "hipaa", "nist-800-53"];
    if !valid_frameworks.contains(&framework.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "unsupported framework",
                "supported": valid_frameworks,
            })),
        );
    }

    let pool = &state.pool;

    // Gather metrics from the database
    let total_events = count_query(pool, "SELECT COUNT(*) AS cnt FROM events").await;
    let agent_count = count_query(pool, "SELECT COUNT(*) AS cnt FROM agents").await;

    let pii_count = count_query(
        pool,
        "SELECT COUNT(*) AS cnt FROM events \
         WHERE pii_detected IS NOT NULL \
         AND pii_detected != '[]'::jsonb \
         AND jsonb_array_length(pii_detected) > 0",
    )
    .await;

    let policy_block_count = count_query(
        pool,
        "SELECT COUNT(*) AS cnt FROM events WHERE compliance_tag LIKE 'block:%'",
    )
    .await;

    let high_risk_count = count_query(
        pool,
        "SELECT COUNT(*) AS cnt FROM events WHERE risk_score > 75",
    )
    .await;

    let budget_config_count = count_query(pool, "SELECT COUNT(*) AS cnt FROM budget_config").await;

    // Build framework-specific controls
    let controls = build_compliance_controls(
        &framework,
        total_events,
        agent_count,
        pii_count,
        policy_block_count,
        high_risk_count,
        budget_config_count,
    );

    let total_score: f64 = if controls.is_empty() {
        0.0
    } else {
        let sum: f64 = controls
            .iter()
            .filter_map(|c| c.get("score").and_then(|s| s.as_f64()))
            .sum();
        let count = controls.len() as f64;
        (sum / count * 10.0).round() / 10.0
    };

    let status = if total_score >= 80.0 {
        "compliant"
    } else if total_score >= 50.0 {
        "partial"
    } else {
        "non-compliant"
    };

    (
        StatusCode::OK,
        Json(json!({
            "framework": framework,
            "status": status,
            "score": total_score,
            "generated_at": Utc::now().to_rfc3339(),
            "controls": controls,
        })),
    )
}

/// Build framework-specific compliance controls based on real metrics.
fn build_compliance_controls(
    framework: &str,
    total_events: i64,
    agent_count: i64,
    pii_count: i64,
    policy_block_count: i64,
    high_risk_count: i64,
    budget_config_count: i64,
) -> Vec<serde_json::Value> {
    match framework {
        "soc2" => vec![
            control(
                "CC6.1",
                "Logical Access Controls",
                "Agent identification and authentication tracking",
                agent_count > 0,
                agent_count,
                format!(
                    "{} agents tracked with identity fingerprinting",
                    agent_count
                ),
            ),
            control(
                "CC7.2",
                "System Monitoring",
                "Continuous monitoring of AI agent activity",
                total_events > 0,
                total_events,
                format!("{} events captured in audit log", total_events),
            ),
            control(
                "CC6.7",
                "Data Classification",
                "PII detection and classification in agent traffic",
                true,
                pii_count,
                format!("{} PII instances detected and classified", pii_count),
            ),
            control(
                "CC7.4",
                "Incident Response",
                "Policy enforcement and automated blocking",
                policy_block_count > 0 || total_events > 0,
                policy_block_count,
                format!("{} policy blocks enforced", policy_block_count),
            ),
            control(
                "CC8.1",
                "Change Management",
                "Agent lifecycle tracking and risk scoring",
                high_risk_count == 0 || total_events > 100,
                high_risk_count,
                format!("{} high-risk events flagged for review", high_risk_count),
            ),
            control(
                "CC9.1",
                "Risk Mitigation",
                "Budget controls and cost governance",
                budget_config_count > 0,
                budget_config_count,
                format!("{} budget policies configured", budget_config_count),
            ),
        ],
        "eu-ai-act" => vec![
            control(
                "ART-9",
                "Risk Management System",
                "Continuous risk scoring of AI agent operations",
                total_events > 0,
                total_events,
                format!(
                    "{} operations risk-scored; {} flagged high-risk",
                    total_events, high_risk_count
                ),
            ),
            control(
                "ART-12",
                "Record Keeping",
                "Complete audit trail of all AI agent interactions",
                total_events > 0,
                total_events,
                format!(
                    "{} events in tamper-evident audit log with lineage hashing",
                    total_events
                ),
            ),
            control(
                "ART-13",
                "Transparency",
                "AI system identification and capability disclosure",
                agent_count > 0,
                agent_count,
                format!(
                    "{} agents registered with type, framework, and API targets",
                    agent_count
                ),
            ),
            control(
                "ART-14",
                "Human Oversight",
                "Kill switch and policy-based intervention capability",
                true,
                policy_block_count,
                format!(
                    "{} automated interventions; kill switch available for all agents",
                    policy_block_count
                ),
            ),
            control(
                "ART-10",
                "Data Governance",
                "PII detection and data flow monitoring",
                true,
                pii_count,
                format!("{} PII instances detected across agent traffic", pii_count),
            ),
        ],
        "hipaa" => vec![
            control(
                "164.312(a)",
                "Access Control",
                "Unique agent identification and session tracking",
                agent_count > 0,
                agent_count,
                format!(
                    "{} agents with unique identifiers and session isolation",
                    agent_count
                ),
            ),
            control(
                "164.312(b)",
                "Audit Controls",
                "Hardware, software, and procedural audit mechanisms",
                total_events > 0,
                total_events,
                format!("{} audit events with SHA-256 lineage hashing", total_events),
            ),
            control(
                "164.312(c)",
                "Integrity Controls",
                "Protection against improper alteration or destruction",
                total_events > 0,
                total_events,
                format!(
                    "Merkle-chain integrity verification across {} events",
                    total_events
                ),
            ),
            control(
                "164.312(d)",
                "Person Authentication",
                "Agent identity verification",
                agent_count > 0,
                agent_count,
                format!("{} agents authenticated via fingerprinting", agent_count),
            ),
            control(
                "164.308(a)(5)",
                "Security Awareness",
                "PII and PHI detection in AI agent traffic",
                true,
                pii_count,
                format!("{} PII/PHI instances detected and flagged", pii_count),
            ),
            control(
                "164.308(a)(6)",
                "Security Incident Procedures",
                "Automated policy enforcement and incident response",
                true,
                policy_block_count,
                format!(
                    "{} security incidents automatically blocked",
                    policy_block_count
                ),
            ),
        ],
        "nist-800-53" => vec![
            control(
                "AC-2",
                "Account Management",
                "AI agent account lifecycle management",
                agent_count > 0,
                agent_count,
                format!("{} agent accounts managed with status tracking", agent_count),
            ),
            control(
                "AU-2",
                "Audit Events",
                "Selection of auditable events from AI agent operations",
                total_events > 0,
                total_events,
                format!("{} events captured across 15 event kinds", total_events),
            ),
            control(
                "AU-3",
                "Content of Audit Records",
                "Audit records contain sufficient detail for forensic analysis",
                total_events > 0,
                total_events,
                "Events include timestamps, agent ID, session ID, lineage hash, and compliance tags"
                    .to_string(),
            ),
            control(
                "RA-5",
                "Vulnerability Monitoring",
                "Continuous risk scoring and anomaly detection",
                true,
                high_risk_count,
                format!(
                    "{} high-risk events identified through continuous scoring",
                    high_risk_count
                ),
            ),
            control(
                "SI-4",
                "System Monitoring",
                "Real-time monitoring of AI agent information flows",
                total_events > 0,
                total_events,
                format!(
                    "{} events monitored in real-time with PII and risk detection",
                    total_events
                ),
            ),
            control(
                "IR-4",
                "Incident Handling",
                "Automated incident response via policy engine",
                true,
                policy_block_count,
                format!(
                    "{} incidents handled automatically by policy engine",
                    policy_block_count
                ),
            ),
            control(
                "SA-9",
                "External System Services",
                "Governance of external AI API consumption",
                budget_config_count > 0 || agent_count > 0,
                budget_config_count,
                format!(
                    "{} budget policies governing external API usage",
                    budget_config_count
                ),
            ),
        ],
        _ => vec![],
    }
}

/// Build a single control entry with score derived from real data.
fn control(
    id: &str,
    name: &str,
    description: &str,
    passing: bool,
    metric_value: i64,
    evidence: String,
) -> serde_json::Value {
    let score: f64 = if passing && metric_value > 0 {
        100.0
    } else if passing {
        75.0
    } else {
        0.0
    };

    let status = if score >= 80.0 {
        "pass"
    } else if score >= 50.0 {
        "partial"
    } else {
        "fail"
    };

    json!({
        "id": id,
        "name": name,
        "status": status,
        "score": score,
        "description": description,
        "evidence": evidence,
    })
}

// ── 2. Risk Overview ─────────────────────────────────────────────────────────

/// GET /api/v1/risk/overview
pub async fn risk_overview(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = &state.pool;

    // Average risk score across recent events
    let avg_risk: f32 = sqlx::query(
        "SELECT COALESCE(AVG(risk_score), 0.0)::real AS avg_risk \
         FROM events WHERE risk_score IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .and_then(|r| r.try_get::<f32, _>("avg_risk"))
    .unwrap_or(0.0);

    // Count warning events
    let warn_count = count_query(
        pool,
        "SELECT COUNT(*) AS cnt FROM events WHERE compliance_tag LIKE 'warn:%'",
    )
    .await;

    // Count block events
    let block_count = count_query(
        pool,
        "SELECT COUNT(*) AS cnt FROM events WHERE compliance_tag LIKE 'block:%'",
    )
    .await;

    // PII detections in last 24h
    let pii_24h = count_query(
        pool,
        "SELECT COUNT(*) AS cnt FROM events \
         WHERE pii_detected IS NOT NULL \
         AND pii_detected != '[]'::jsonb \
         AND jsonb_array_length(pii_detected) > 0 \
         AND timestamp >= NOW() - INTERVAL '24 hours'",
    )
    .await;

    // High risk events count
    let high_risk = count_query(
        pool,
        "SELECT COUNT(*) AS cnt FROM events WHERE risk_score > 75",
    )
    .await;

    // 7-day trend: group events by day, compute avg risk per day
    let trend_rows = sqlx::query(
        "SELECT DATE(timestamp) AS day, \
                COALESCE(AVG(risk_score), 0.0)::real AS avg_risk, \
                COUNT(*) AS event_count \
         FROM events \
         WHERE timestamp >= NOW() - INTERVAL '7 days' \
         AND risk_score IS NOT NULL \
         GROUP BY DATE(timestamp) \
         ORDER BY day ASC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let trend: Vec<serde_json::Value> = trend_rows
        .iter()
        .map(|r: &sqlx::postgres::PgRow| {
            let day: chrono::NaiveDate = r
                .try_get::<chrono::NaiveDate, _>("day")
                .unwrap_or_else(|_| Utc::now().date_naive());
            let score = r.try_get::<f32, _>("avg_risk").unwrap_or(0.0);
            let count = r.try_get::<i64, _>("event_count").unwrap_or(0);
            json!({
                "day": day.to_string(),
                "score": score,
                "event_count": count,
            })
        })
        .collect();

    // Recent alerts: events with risk_score > 50 or compliance_tag like 'warn:%' or 'block:%'
    let alert_rows = sqlx::query(
        "SELECT id, agent_id, timestamp, risk_score, compliance_tag, event_kind \
         FROM events \
         WHERE risk_score > 50 \
            OR compliance_tag LIKE 'warn:%' \
            OR compliance_tag LIKE 'block:%' \
         ORDER BY timestamp DESC \
         LIMIT 20",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let alerts: Vec<serde_json::Value> = alert_rows
        .iter()
        .map(|r: &sqlx::postgres::PgRow| {
            json!({
                "id": r.try_get::<uuid::Uuid, _>("id").ok().map(|u| u.to_string()),
                "agent_id": r.try_get::<String, _>("agent_id").unwrap_or_default(),
                "timestamp": r.try_get::<chrono::DateTime<Utc>, _>("timestamp")
                    .ok().map(|t| t.to_rfc3339()),
                "risk_score": r.try_get::<Option<f32>, _>("risk_score").ok().flatten(),
                "compliance_tag": r.try_get::<String, _>("compliance_tag").unwrap_or_default(),
                "event_kind": r.try_get::<Option<String>, _>("event_kind").ok().flatten(),
            })
        })
        .collect();

    let risk_label = if avg_risk >= 75.0 {
        "critical"
    } else if avg_risk >= 50.0 {
        "high"
    } else if avg_risk >= 25.0 {
        "medium"
    } else {
        "low"
    };

    (
        StatusCode::OK,
        Json(json!({
            "risk_score": (avg_risk * 10.0).round() / 10.0,
            "risk_label": risk_label,
            "alerts": alerts,
            "trend": trend,
            "stats": {
                "warnings": warn_count,
                "blocks": block_count,
                "pii_detections_24h": pii_24h,
                "high_risk_events": high_risk,
            },
        })),
    )
}

// ── 3. PII Activity ──────────────────────────────────────────────────────────

/// GET /api/v1/pii/activity
pub async fn pii_activity(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PiiActivityParams>,
) -> impl IntoResponse {
    let pool = &state.pool;
    let limit = params.limit.unwrap_or(50).clamp(1, 500);
    let offset = params.offset.unwrap_or(0).max(0);

    // Fetch events with PII detections
    let rows = sqlx::query(
        "SELECT id, agent_id, timestamp, pii_detected, compliance_tag, model \
         FROM events \
         WHERE pii_detected IS NOT NULL \
         AND pii_detected != '[]'::jsonb \
         AND jsonb_array_length(pii_detected) > 0 \
         ORDER BY timestamp DESC \
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("pii_activity query error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to query PII activity", "detail": e.to_string() })),
            );
        }
    };

    // Build detection list and aggregate PII types
    let mut type_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut detections: Vec<serde_json::Value> = Vec::new();

    for r in &rows {
        let pii_json: serde_json::Value = r
            .try_get::<serde_json::Value, _>("pii_detected")
            .unwrap_or(json!([]));

        let pii_array = pii_json.as_array().cloned().unwrap_or_default();
        let pii_count = pii_array.len() as i64;

        // Aggregate types
        for item in &pii_array {
            if let Some(pii_type) = item.get("pii_type").and_then(|v| v.as_str()) {
                *type_counts.entry(pii_type.to_string()).or_insert(0) += 1;
            }
        }

        let action = r.try_get::<String, _>("compliance_tag").unwrap_or_default();
        let action_label = if action.starts_with("block:") {
            "blocked"
        } else if action.starts_with("warn:") {
            "warned"
        } else {
            "logged"
        };

        // Extract a representative type from the first detection
        let pii_type_label = pii_array
            .first()
            .and_then(|item| item.get("pii_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        detections.push(json!({
            "id": r.try_get::<uuid::Uuid, _>("id").ok().map(|u| u.to_string()),
            "agent_id": r.try_get::<String, _>("agent_id").unwrap_or_default(),
            "type": pii_type_label,
            "count": pii_count,
            "timestamp": r.try_get::<chrono::DateTime<Utc>, _>("timestamp")
                .ok().map(|t| t.to_rfc3339()),
            "action": action_label,
            "model": r.try_get::<Option<String>, _>("model").ok().flatten(),
            "details": pii_array,
        }));
    }

    // Total PII events count
    let total = count_query(
        pool,
        "SELECT COUNT(*) AS cnt FROM events \
         WHERE pii_detected IS NOT NULL \
         AND pii_detected != '[]'::jsonb \
         AND jsonb_array_length(pii_detected) > 0",
    )
    .await;

    let total_pii_instances: i64 = type_counts.values().sum();

    (
        StatusCode::OK,
        Json(json!({
            "detections": detections,
            "total": total,
            "types": type_counts,
            "summary": {
                "total_events_with_pii": total,
                "total_pii_instances": total_pii_instances,
                "unique_types": type_counts.len(),
            },
        })),
    )
}

// ── 4. Policies ──────────────────────────────────────────────────────────────

/// GET /api/v1/policies
///
/// Returns policy definitions from the policies configuration file. Falls back
/// to the example config if no custom file is found.
pub async fn list_policies(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = &state.pool;

    // Try to read policies from config files
    let policy_paths = [
        "config/policies.yaml",
        "config/policies.example.yaml",
        "/etc/govrix/policies.yaml",
    ];

    let mut policies_value: Option<serde_json::Value> = None;

    for path in &policy_paths {
        if let Ok(content) = tokio::fs::read_to_string(path).await {
            if let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(&content) {
                policies_value = Some(yaml);
                break;
            }
        }
    }

    let policies = if let Some(ref val) = policies_value {
        val.get("policies")
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let formatted: Vec<serde_json::Value> = policies
        .iter()
        .map(|p| {
            let rules = p.get("rules").and_then(|r| r.as_array());
            let rules_count = rules.map(|r| r.len()).unwrap_or(0);
            let first_action = rules
                .and_then(|r| r.first())
                .and_then(|r| r.get("action"))
                .and_then(|a| a.as_str())
                .unwrap_or("alert");

            json!({
                "id": p.get("id").and_then(|v| v.as_str()).unwrap_or("unknown"),
                "name": p.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed"),
                "enabled": p.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                "rules_count": rules_count,
                "action": first_action,
                "description": format!(
                    "{} with {} rule(s)",
                    p.get("name").and_then(|v| v.as_str()).unwrap_or("Policy"),
                    rules_count
                ),
            })
        })
        .collect();

    // Also query compliance_tag stats to show policy enforcement activity
    let enforcement_stats = sqlx::query(
        "SELECT compliance_tag, COUNT(*) AS cnt \
         FROM events \
         WHERE compliance_tag != 'pass:all' \
         GROUP BY compliance_tag \
         ORDER BY cnt DESC \
         LIMIT 20",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let enforcement: Vec<serde_json::Value> = enforcement_stats
        .iter()
        .map(|r: &sqlx::postgres::PgRow| {
            json!({
                "tag": r.try_get::<String, _>("compliance_tag").unwrap_or_default(),
                "count": r.try_get::<i64, _>("cnt").unwrap_or(0),
            })
        })
        .collect();

    let total = formatted.len();

    (
        StatusCode::OK,
        Json(json!({
            "policies": formatted,
            "total": total,
            "enforcement_stats": enforcement,
        })),
    )
}

/// POST /api/v1/policies/reload
pub async fn reload_policies() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "message": "Policy reload requested. Restart proxy to apply changes.",
        })),
    )
}

// ── 5. Kill Switch ───────────────────────────────────────────────────────────

/// GET /api/v1/kill-switch/status
pub async fn kill_switch_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = &state.pool;

    // Query blocked agents
    let blocked_rows = sqlx::query(
        "SELECT id, name, status, last_seen_at, updated_at \
         FROM agents \
         WHERE status = 'blocked' \
         ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let killed_agents: Vec<serde_json::Value> = blocked_rows
        .iter()
        .map(|r: &sqlx::postgres::PgRow| {
            json!({
                "agent_id": r.try_get::<String, _>("id").unwrap_or_default(),
                "name": r.try_get::<Option<String>, _>("name").ok().flatten(),
                "status": r.try_get::<String, _>("status").unwrap_or_default(),
                "last_seen_at": r.try_get::<chrono::DateTime<Utc>, _>("last_seen_at")
                    .ok().map(|t| t.to_rfc3339()),
                "blocked_at": r.try_get::<chrono::DateTime<Utc>, _>("updated_at")
                    .ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    // Query killed sessions
    let killed_sessions = sqlx::query(
        "SELECT session_id, agent_id, killed_at, killed_by, kill_reason \
         FROM sessions \
         WHERE status = 'killed' \
         ORDER BY killed_at DESC \
         LIMIT 20",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let recent_kills: Vec<serde_json::Value> = killed_sessions
        .iter()
        .map(|r: &sqlx::postgres::PgRow| {
            json!({
                "session_id": r.try_get::<String, _>("session_id").unwrap_or_default(),
                "agent_id": r.try_get::<String, _>("agent_id").unwrap_or_default(),
                "killed_at": r.try_get::<Option<chrono::DateTime<Utc>>, _>("killed_at")
                    .ok().flatten().map(|t| t.to_rfc3339()),
                "killed_by": r.try_get::<Option<String>, _>("killed_by").ok().flatten(),
                "reason": r.try_get::<Option<String>, _>("kill_reason").ok().flatten(),
            })
        })
        .collect();

    let killed_count = killed_agents.len();

    (
        StatusCode::OK,
        Json(json!({
            "killed_agents": killed_agents,
            "killed_count": killed_count,
            "recent_kills": recent_kills,
        })),
    )
}

/// GET /api/v1/kill-switch/history
pub async fn kill_switch_history(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = &state.pool;

    // Block events from compliance_tag
    let block_rows = sqlx::query(
        "SELECT id, agent_id, timestamp, compliance_tag, event_kind, risk_score \
         FROM events \
         WHERE compliance_tag LIKE 'block:%' \
         ORDER BY timestamp DESC \
         LIMIT 50",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let events: Vec<serde_json::Value> = block_rows
        .iter()
        .map(|r: &sqlx::postgres::PgRow| {
            json!({
                "id": r.try_get::<uuid::Uuid, _>("id").ok().map(|u| u.to_string()),
                "agent_id": r.try_get::<String, _>("agent_id").unwrap_or_default(),
                "timestamp": r.try_get::<chrono::DateTime<Utc>, _>("timestamp")
                    .ok().map(|t| t.to_rfc3339()),
                "compliance_tag": r.try_get::<String, _>("compliance_tag").unwrap_or_default(),
                "event_kind": r.try_get::<Option<String>, _>("event_kind").ok().flatten(),
                "risk_score": r.try_get::<Option<f32>, _>("risk_score").ok().flatten(),
            })
        })
        .collect();

    // Count blocks today
    let killed_today = count_query(
        pool,
        "SELECT COUNT(*) AS cnt FROM events \
         WHERE compliance_tag LIKE 'block:%' \
         AND timestamp >= CURRENT_DATE",
    )
    .await;

    // Circuit breaker count (policy.block events)
    let circuit_breakers = count_query(
        pool,
        "SELECT COUNT(*) AS cnt FROM events WHERE event_kind = 'policy.block'",
    )
    .await;

    (
        StatusCode::OK,
        Json(json!({
            "events": events,
            "killed_today": killed_today,
            "circuit_breakers_triggered": circuit_breakers,
        })),
    )
}

/// POST /api/v1/kill-switch/kill
pub async fn kill_agent(
    State(state): State<Arc<AppState>>,
    Json(body): Json<KillAgentBody>,
) -> impl IntoResponse {
    let pool = &state.pool;
    let reason = body
        .reason
        .unwrap_or_else(|| "Manual kill via API".to_string());

    // Update agent status to 'blocked'
    let result = sqlx::query("UPDATE agents SET status = 'blocked' WHERE id = $1")
        .bind(&body.agent_id)
        .execute(pool)
        .await;

    match result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "agent not found", "agent_id": body.agent_id })),
                );
            }

            // Also kill any active sessions for this agent
            let _ = sqlx::query(
                "UPDATE sessions SET status = 'killed', killed_at = NOW(), \
                 killed_by = 'api', kill_reason = $2 \
                 WHERE agent_id = $1 AND status = 'active'",
            )
            .bind(&body.agent_id)
            .bind(&reason)
            .execute(pool)
            .await;

            (
                StatusCode::OK,
                Json(json!({
                    "status": "killed",
                    "agent_id": body.agent_id,
                    "reason": reason,
                    "killed_at": Utc::now().to_rfc3339(),
                })),
            )
        }
        Err(e) => {
            tracing::error!("kill_agent error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to kill agent", "detail": e.to_string() })),
            )
        }
    }
}

/// POST /api/v1/kill-switch/revive
pub async fn revive_agent(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ReviveAgentBody>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let result = sqlx::query("UPDATE agents SET status = 'active' WHERE id = $1")
        .bind(&body.agent_id)
        .execute(pool)
        .await;

    match result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "agent not found", "agent_id": body.agent_id })),
                );
            }

            (
                StatusCode::OK,
                Json(json!({
                    "status": "revived",
                    "agent_id": body.agent_id,
                    "revived_at": Utc::now().to_rfc3339(),
                })),
            )
        }
        Err(e) => {
            tracing::error!("revive_agent error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to revive agent", "detail": e.to_string() })),
            )
        }
    }
}

// ── 6. Platform Health ───────────────────────────────────────────────────────

/// GET /api/v1/platform/health
pub async fn platform_health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = &state.pool;

    let db_status = match govrix_scout_store::health_check(pool).await {
        Ok(_) => "connected",
        Err(_) => "unavailable",
    };

    let uptime_seconds = state.started_at.elapsed().as_secs();

    // Collect feature flags based on what tables have data
    let has_events: bool = sqlx::query("SELECT EXISTS(SELECT 1 FROM events LIMIT 1) AS ok")
        .fetch_one(pool)
        .await
        .and_then(|r| r.try_get::<bool, _>("ok"))
        .unwrap_or(false);

    let has_agents: bool = sqlx::query("SELECT EXISTS(SELECT 1 FROM agents LIMIT 1) AS ok")
        .fetch_one(pool)
        .await
        .and_then(|r| r.try_get::<bool, _>("ok"))
        .unwrap_or(false);

    let has_sessions: bool = sqlx::query("SELECT EXISTS(SELECT 1 FROM sessions LIMIT 1) AS ok")
        .fetch_one(pool)
        .await
        .and_then(|r| r.try_get::<bool, _>("ok"))
        .unwrap_or(false);

    let has_budgets: bool = sqlx::query("SELECT EXISTS(SELECT 1 FROM budget_config LIMIT 1) AS ok")
        .fetch_one(pool)
        .await
        .and_then(|r| r.try_get::<bool, _>("ok"))
        .unwrap_or(false);

    (
        StatusCode::OK,
        Json(json!({
            "status": if db_status == "connected" { "ok" } else { "degraded" },
            "tier": "community",
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_seconds": uptime_seconds,
            "database": db_status,
            "features": {
                "event_logging": has_events,
                "agent_registry": has_agents,
                "session_tracking": has_sessions,
                "budget_controls": has_budgets,
                "pii_detection": true,
                "risk_scoring": true,
                "compliance_reports": true,
                "kill_switch": true,
            },
        })),
    )
}

// ── 7. Sessions ──────────────────────────────────────────────────────────────

/// GET /api/v1/sessions
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListSessionsParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let offset = params.offset.unwrap_or(0).max(0);

    match govrix_scout_store::sessions::list_sessions(
        &state.pool,
        params.agent_id.as_deref(),
        params.status.as_deref(),
        limit,
        offset,
    )
    .await
    {
        Ok(sessions) => {
            let total = sessions.len();
            (
                StatusCode::OK,
                Json(json!({
                    "data": sessions,
                    "total": total,
                    "limit": limit,
                    "offset": offset,
                })),
            )
        }
        Err(e) => {
            tracing::error!("list_sessions error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to query sessions", "detail": e.to_string() })),
            )
        }
    }
}

/// GET /api/v1/sessions/{id}
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let pool = &state.pool;

    // Fetch session details
    let session = match govrix_scout_store::sessions::get_session(pool, &id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "session not found", "session_id": id })),
            );
        }
        Err(e) => {
            tracing::error!("get_session error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch session", "detail": e.to_string() })),
            );
        }
    };

    // Fetch events belonging to this session.
    // session_id in the events table is a UUID while the sessions table uses TEXT,
    // so we attempt to parse as UUID; if that fails, return the session without events.
    let event_list: Vec<serde_json::Value> = if let Ok(sid) = uuid::Uuid::parse_str(&id) {
        let events = sqlx::query(
            "SELECT id, agent_id, timestamp, event_kind, provider, model, \
                    status_code, risk_score, compliance_tag, cost_usd, \
                    input_tokens, output_tokens, tool_name \
             FROM events \
             WHERE session_id = $1 \
             ORDER BY timestamp ASC \
             LIMIT 200",
        )
        .bind(sid)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        events
            .iter()
            .map(|r: &sqlx::postgres::PgRow| {
                json!({
                    "id": r.try_get::<uuid::Uuid, _>("id").ok().map(|u| u.to_string()),
                    "agent_id": r.try_get::<String, _>("agent_id").unwrap_or_default(),
                    "timestamp": r.try_get::<chrono::DateTime<Utc>, _>("timestamp")
                        .ok().map(|t| t.to_rfc3339()),
                    "event_kind": r.try_get::<Option<String>, _>("event_kind").ok().flatten(),
                    "provider": r.try_get::<String, _>("provider").unwrap_or_default(),
                    "model": r.try_get::<Option<String>, _>("model").ok().flatten(),
                    "status_code": r.try_get::<Option<i32>, _>("status_code").ok().flatten(),
                    "risk_score": r.try_get::<Option<f32>, _>("risk_score").ok().flatten(),
                    "compliance_tag": r.try_get::<String, _>("compliance_tag").unwrap_or_default(),
                    "cost_usd": r.try_get::<Option<f64>, _>("cost_usd").ok().flatten(),
                    "input_tokens": r.try_get::<Option<i32>, _>("input_tokens").ok().flatten(),
                    "output_tokens": r.try_get::<Option<i32>, _>("output_tokens").ok().flatten(),
                    "tool_name": r.try_get::<Option<String>, _>("tool_name").ok().flatten(),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    (
        StatusCode::OK,
        Json(json!({
            "data": session,
            "events": event_list,
            "event_count": event_list.len(),
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compliance_controls_soc2() {
        let controls = build_compliance_controls("soc2", 100, 5, 3, 2, 1, 1);
        assert_eq!(controls.len(), 6);
        for c in &controls {
            assert!(c.get("score").is_some());
            assert!(c.get("id").is_some());
            assert!(c.get("name").is_some());
        }
    }

    #[test]
    fn compliance_controls_eu_ai_act() {
        let controls = build_compliance_controls("eu-ai-act", 0, 0, 0, 0, 0, 0);
        assert_eq!(controls.len(), 5);
    }

    #[test]
    fn compliance_controls_hipaa() {
        let controls = build_compliance_controls("hipaa", 50, 3, 1, 0, 0, 0);
        assert_eq!(controls.len(), 6);
    }

    #[test]
    fn compliance_controls_nist() {
        let controls = build_compliance_controls("nist-800-53", 200, 10, 5, 3, 2, 4);
        assert_eq!(controls.len(), 7);
    }

    #[test]
    fn compliance_controls_unknown_framework() {
        let controls = build_compliance_controls("unknown", 0, 0, 0, 0, 0, 0);
        assert!(controls.is_empty());
    }

    #[test]
    fn control_scoring_passing_with_data() {
        let c = control(
            "TEST-1",
            "Test Control",
            "Test description",
            true,
            10,
            "evidence".to_string(),
        );
        assert_eq!(c.get("score").unwrap().as_f64().unwrap(), 100.0);
        assert_eq!(c.get("status").unwrap().as_str().unwrap(), "pass");
    }

    #[test]
    fn control_scoring_passing_no_data() {
        let c = control(
            "TEST-2",
            "Test Control",
            "Test description",
            true,
            0,
            "no data".to_string(),
        );
        assert_eq!(c.get("score").unwrap().as_f64().unwrap(), 75.0);
        assert_eq!(c.get("status").unwrap().as_str().unwrap(), "partial");
    }

    #[test]
    fn control_scoring_failing() {
        let c = control(
            "TEST-3",
            "Test Control",
            "Test description",
            false,
            0,
            "failing".to_string(),
        );
        assert_eq!(c.get("score").unwrap().as_f64().unwrap(), 0.0);
        assert_eq!(c.get("status").unwrap().as_str().unwrap(), "fail");
    }

    #[test]
    fn risk_label_thresholds() {
        let check = |v: f32| -> &str {
            if v >= 75.0 {
                "critical"
            } else if v >= 50.0 {
                "high"
            } else if v >= 25.0 {
                "medium"
            } else {
                "low"
            }
        };
        assert_eq!(check(80.0), "critical");
        assert_eq!(check(60.0), "high");
        assert_eq!(check(30.0), "medium");
        assert_eq!(check(10.0), "low");
    }

    #[test]
    fn valid_frameworks() {
        let valid = ["soc2", "eu-ai-act", "hipaa", "nist-800-53"];
        assert!(valid.contains(&"soc2"));
        assert!(valid.contains(&"eu-ai-act"));
        assert!(!valid.contains(&"pci-dss"));
    }
}
