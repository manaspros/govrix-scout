//! Trace model — maps to the `traces` table (migration 011).
//!
//! A trace represents the full lifecycle of a single top-level agent task:
//! from the first request through all sub-agent delegations to completion.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// A full agent task trace record.
///
/// Maps to the `traces` table. Traces are created lazily on the first event
/// for a new session and updated asynchronously by the background writer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    /// Primary key — Govrix internal trace UUID.
    pub trace_id: Uuid,

    /// The originating agent that started this trace (FK → agents.id).
    pub root_agent_id: String,

    /// First prompt text, truncated to 500 chars (optional).
    pub task_description: Option<String>,

    /// Current lifecycle status.
    pub status: TraceStatus,

    /// Who stopped the trace (set when stopped/failed).
    /// Values: "circuit_breaker", "kill_switch", "user", "budget".
    pub stopped_by: Option<String>,

    /// Error message if status = 'failed'.
    pub error_message: Option<String>,

    /// When the trace started.
    pub started_at: DateTime<Utc>,

    /// When the trace completed (None if still running).
    pub completed_at: Option<DateTime<Utc>>,

    /// When this record was created.
    pub created_at: DateTime<Utc>,

    /// Accumulated cost across all events in this trace (USD).
    pub total_cost_usd: f64,

    /// Highest risk_score seen across all events in this trace.
    pub peak_risk_score: Option<f32>,

    /// Total number of events in this trace.
    pub event_count: i32,

    /// Number of distinct agents observed within this trace.
    pub agent_count: i32,

    /// W3C traceparent hex trace-id for correlation with external tools (Datadog, Jaeger).
    pub external_trace_id: Option<String>,

    /// Arbitrary metadata JSONB.
    pub metadata: Option<JsonValue>,
}

/// Trace lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceStatus {
    Running,
    Completed,
    Stopped,
    Failed,
}

impl std::fmt::Display for TraceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TraceStatus::Running => "running",
            TraceStatus::Completed => "completed",
            TraceStatus::Stopped => "stopped",
            TraceStatus::Failed => "failed",
        };
        write!(f, "{}", s)
    }
}

impl Trace {
    /// Create a new running trace for the given root agent.
    pub fn new(root_agent_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            trace_id: Uuid::now_v7(),
            root_agent_id: root_agent_id.into(),
            task_description: None,
            status: TraceStatus::Running,
            stopped_by: None,
            error_message: None,
            started_at: now,
            completed_at: None,
            created_at: now,
            total_cost_usd: 0.0,
            peak_risk_score: None,
            event_count: 0,
            agent_count: 1,
            external_trace_id: None,
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_new_defaults() {
        let t = Trace::new("agent-001");
        assert_eq!(t.status, TraceStatus::Running);
        assert_eq!(t.event_count, 0);
        assert_eq!(t.agent_count, 1);
        assert!(t.completed_at.is_none());
        assert!(t.peak_risk_score.is_none());
    }

    #[test]
    fn trace_status_display() {
        assert_eq!(TraceStatus::Running.to_string(), "running");
        assert_eq!(TraceStatus::Failed.to_string(), "failed");
    }
}
