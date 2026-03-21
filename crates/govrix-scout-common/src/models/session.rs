//! Persistent session model — maps to the `sessions` table (migration 010).
//!
//! Sessions group related agent requests across restarts. The in-memory session
//! map is backed by this table and rebuilt at startup from active rows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A persistent agent session record.
///
/// Maps to the `sessions` table. Session IDs are TEXT (not UUID) because they
/// are derived deterministically from available request context rather than
/// generated randomly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Deterministic session identifier (see spec §A.4 for derivation priority).
    pub session_id: String,

    /// The agent this session belongs to (FK → agents.id).
    pub agent_id: String,

    /// Internal trace UUID for this session (None until first event creates the trace).
    pub trace_id: Option<Uuid>,

    /// Current lifecycle status.
    pub status: SessionStatus,

    /// When this session was first seen.
    pub started_at: DateTime<Utc>,

    /// When the most recent event in this session arrived.
    pub last_event_at: DateTime<Utc>,

    /// When this record was created.
    pub created_at: DateTime<Utc>,

    /// Total number of events in this session.
    pub event_count: i32,

    /// Accumulated cost for this session (USD).
    pub total_cost_usd: f64,

    /// When the session was killed (None if not killed).
    pub killed_at: Option<DateTime<Utc>>,

    /// Who triggered the kill: "user", "budget", "risk_threshold", "loop_detector", "timeout".
    pub killed_by: Option<String>,

    /// Human-readable kill reason.
    pub kill_reason: Option<String>,
}

/// Session lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Idle,
    Completed,
    Killed,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SessionStatus::Active => "active",
            SessionStatus::Idle => "idle",
            SessionStatus::Completed => "completed",
            SessionStatus::Killed => "killed",
        };
        write!(f, "{}", s)
    }
}

impl Session {
    /// Create a new active session.
    pub fn new(session_id: impl Into<String>, agent_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            session_id: session_id.into(),
            agent_id: agent_id.into(),
            trace_id: None,
            status: SessionStatus::Active,
            started_at: now,
            last_event_at: now,
            created_at: now,
            event_count: 0,
            total_cost_usd: 0.0,
            killed_at: None,
            killed_by: None,
            kill_reason: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_new_defaults() {
        let s = Session::new("sess-001", "agent-001");
        assert_eq!(s.status, SessionStatus::Active);
        assert_eq!(s.event_count, 0);
        assert!(s.trace_id.is_none());
        assert!(s.killed_at.is_none());
    }

    #[test]
    fn session_status_display() {
        assert_eq!(SessionStatus::Active.to_string(), "active");
        assert_eq!(SessionStatus::Killed.to_string(), "killed");
    }
}
