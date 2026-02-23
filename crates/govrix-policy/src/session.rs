//! Session Recorder — cryptographically signed replay of agent sessions.
//!
//! Enterprise feature: collects events by `session_id` and produces a
//! SHA-256 integrity-sealed transcript suitable for audit and compliance.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A single recorded event within a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub timestamp: String,
    pub agent_id: String,
    pub method: String,
    pub upstream_target: String,
    pub compliance_tag: String,
    pub tokens_in: Option<i64>,
    pub tokens_out: Option<i64>,
    pub cost_usd: Option<f64>,
    pub status_code: Option<u16>,
    pub lineage_hash: String,
}

/// A complete session recording with integrity verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecording {
    pub session_id: String,
    pub agent_id: String,
    pub started_at: String,
    pub ended_at: String,
    pub events: Vec<SessionEvent>,
    pub event_count: usize,
    pub total_tokens: i64,
    pub total_cost_usd: f64,
    /// SHA-256 hash of the serialized events array -- integrity seal.
    pub integrity_hash: String,
}

impl SessionRecording {
    /// Build a session recording from a list of events sharing the same `session_id`.
    pub fn from_events(session_id: String, agent_id: String, events: Vec<SessionEvent>) -> Self {
        let event_count = events.len();
        let total_tokens: i64 = events
            .iter()
            .map(|e| e.tokens_in.unwrap_or(0) + e.tokens_out.unwrap_or(0))
            .sum();
        let total_cost_usd: f64 = events.iter().map(|e| e.cost_usd.unwrap_or(0.0)).sum();

        let started_at = events
            .first()
            .map(|e| e.timestamp.clone())
            .unwrap_or_default();
        let ended_at = events
            .last()
            .map(|e| e.timestamp.clone())
            .unwrap_or_default();

        // Compute integrity hash over serialized events.
        let events_json = serde_json::to_string(&events).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(events_json.as_bytes());
        let integrity_hash = format!("{:x}", hasher.finalize());

        Self {
            session_id,
            agent_id,
            started_at,
            ended_at,
            events,
            event_count,
            total_tokens,
            total_cost_usd,
            integrity_hash,
        }
    }

    /// Verify the integrity of this recording by recomputing the hash.
    pub fn verify_integrity(&self) -> bool {
        let events_json = serde_json::to_string(&self.events).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(events_json.as_bytes());
        let computed = format!("{:x}", hasher.finalize());
        computed == self.integrity_hash
    }

    /// Render the session as a human-readable markdown transcript.
    pub fn to_markdown(&self) -> String {
        let mut md = format!("# Session Recording: {}\n\n", self.session_id);
        md.push_str(&format!("**Agent:** {}\n", self.agent_id));
        md.push_str(&format!(
            "**Period:** {} -> {}\n",
            self.started_at, self.ended_at
        ));
        md.push_str(&format!(
            "**Events:** {} | **Tokens:** {} | **Cost:** ${:.4}\n",
            self.event_count, self.total_tokens, self.total_cost_usd
        ));
        md.push_str(&format!(
            "**Integrity Hash:** `{}`\n\n",
            self.integrity_hash
        ));
        md.push_str("---\n\n");

        for (i, event) in self.events.iter().enumerate() {
            md.push_str(&format!("### Event {} -- {}\n", i + 1, event.timestamp));
            md.push_str(&format!(
                "- **Method:** {} {}\n",
                event.method, event.upstream_target
            ));
            md.push_str(&format!("- **Compliance:** {}\n", event.compliance_tag));
            if let Some(tokens) = event.tokens_in {
                md.push_str(&format!(
                    "- **Tokens in/out:** {}/{}\n",
                    tokens,
                    event.tokens_out.unwrap_or(0)
                ));
            }
            if let Some(status) = event.status_code {
                md.push_str(&format!("- **Status:** {}\n", status));
            }
            md.push_str(&format!("- **Lineage:** `{}`\n\n", event.lineage_hash));
        }
        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(ts: &str, tokens_in: i64, tokens_out: i64, cost: f64) -> SessionEvent {
        SessionEvent {
            timestamp: ts.to_string(),
            agent_id: "test-agent".to_string(),
            method: "POST".to_string(),
            upstream_target: "https://api.openai.com/v1/chat/completions".to_string(),
            compliance_tag: "pass:all".to_string(),
            tokens_in: Some(tokens_in),
            tokens_out: Some(tokens_out),
            cost_usd: Some(cost),
            status_code: Some(200),
            lineage_hash: "abc123".to_string(),
        }
    }

    #[test]
    fn test_recording_from_events() {
        let events = vec![
            make_event("2026-02-19T10:00:00Z", 100, 50, 0.0015),
            make_event("2026-02-19T10:00:05Z", 200, 100, 0.003),
        ];
        let recording =
            SessionRecording::from_events("sess-001".to_string(), "test-agent".to_string(), events);

        assert_eq!(recording.event_count, 2);
        assert_eq!(recording.total_tokens, 450); // 100+50 + 200+100
        assert!((recording.total_cost_usd - 0.0045).abs() < 1e-10);
        assert_eq!(recording.started_at, "2026-02-19T10:00:00Z");
        assert_eq!(recording.ended_at, "2026-02-19T10:00:05Z");
        assert_eq!(recording.session_id, "sess-001");
        assert_eq!(recording.agent_id, "test-agent");
        assert!(!recording.integrity_hash.is_empty());
    }

    #[test]
    fn test_integrity_verification() {
        let events = vec![
            make_event("2026-02-19T10:00:00Z", 100, 50, 0.0015),
            make_event("2026-02-19T10:00:05Z", 200, 100, 0.003),
        ];
        let recording =
            SessionRecording::from_events("sess-002".to_string(), "test-agent".to_string(), events);

        assert!(recording.verify_integrity());
    }

    #[test]
    fn test_integrity_tampered() {
        let events = vec![
            make_event("2026-02-19T10:00:00Z", 100, 50, 0.0015),
            make_event("2026-02-19T10:00:05Z", 200, 100, 0.003),
        ];
        let mut recording =
            SessionRecording::from_events("sess-003".to_string(), "test-agent".to_string(), events);

        // Tamper with the events after recording was built.
        recording.events[0].tokens_in = Some(9999);

        assert!(!recording.verify_integrity());
    }

    #[test]
    fn test_empty_session() {
        let recording = SessionRecording::from_events(
            "sess-empty".to_string(),
            "test-agent".to_string(),
            vec![],
        );

        assert_eq!(recording.event_count, 0);
        assert_eq!(recording.total_tokens, 0);
        assert!((recording.total_cost_usd).abs() < 1e-10);
        assert_eq!(recording.started_at, "");
        assert_eq!(recording.ended_at, "");
        assert!(recording.verify_integrity());
    }

    #[test]
    fn test_to_markdown() {
        let events = vec![
            make_event("2026-02-19T10:00:00Z", 100, 50, 0.0015),
            make_event("2026-02-19T10:00:05Z", 200, 100, 0.003),
        ];
        let recording =
            SessionRecording::from_events("sess-md".to_string(), "test-agent".to_string(), events);

        let md = recording.to_markdown();
        assert!(md.contains("# Session Recording: sess-md"));
        assert!(md.contains("**Agent:** test-agent"));
        assert!(md.contains("### Event 1"));
        assert!(md.contains("### Event 2"));
        assert!(md.contains("POST https://api.openai.com/v1/chat/completions"));
        assert!(md.contains("pass:all"));
        assert!(md.contains(&recording.integrity_hash));
    }
}
