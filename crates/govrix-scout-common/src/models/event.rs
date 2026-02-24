//! AgentEvent — the core intercepted request/response event.
//!
//! Schema matches the `events` table in MEMORY.md:
//!   id, session_id, agent_id, kind, protocol, upstream_target, timestamp, model,
//!   input/output/total_tokens, cost_usd, latency_ms, status_code, finish_reason,
//!   payload(JSONB), raw_size_bytes, tags(JSONB), error_message, created_at
//!
//! Compliance fields (compliance-first skill):
//!   session_id, timestamp, lineage_hash, compliance_tag — ALL REQUIRED.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Direction of the intercepted traffic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventDirection {
    Outbound,
    Inbound,
}

impl std::fmt::Display for EventDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventDirection::Outbound => write!(f, "outbound"),
            EventDirection::Inbound => write!(f, "inbound"),
        }
    }
}

/// Provider classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    OpenAI,
    Anthropic,
    Google,
    Mcp,
    A2A,
    Custom,
    Unknown,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Provider::OpenAI => "openai",
            Provider::Anthropic => "anthropic",
            Provider::Google => "google",
            Provider::Mcp => "mcp",
            Provider::A2A => "a2a",
            Provider::Custom => "custom",
            Provider::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

/// A single PII finding (type + location, NEVER the actual value).
///
/// Compliance rule: never store actual PII — only the type and location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiDetection {
    /// The type of PII found (e.g. "EMAIL_ADDRESS", "PHONE_NUMBER", "PERSON").
    pub pii_type: String,
    /// Human-readable location description (e.g. "request.messages[1].content offset 42").
    pub location: String,
    /// Confidence score in [0.0, 1.0].
    pub confidence: f32,
}

/// Core intercepted event — maps to the `events` TimescaleDB table.
///
/// Every event MUST have the four compliance fields (compliance-first skill):
///   - `session_id`
///   - `timestamp`
///   - `lineage_hash`
///   - `compliance_tag`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    // ── Identity ──────────────────────────────────────────────────────────────
    /// Primary key (UUIDv7, time-ordered).
    pub id: Uuid,

    /// Compliance: groups related actions in a single agent session/conversation.
    /// Assigned by SessionTracker, NOT random per-request.
    pub session_id: Uuid,

    /// The agent that generated this event (matches agents.id).
    pub agent_id: String,

    // ── Timing ────────────────────────────────────────────────────────────────
    /// Compliance: exact time the action was intercepted (UTC, millisecond precision).
    pub timestamp: DateTime<Utc>,

    /// Round-trip latency in milliseconds.
    pub latency_ms: Option<u32>,

    // ── Request metadata ──────────────────────────────────────────────────────
    /// Direction of traffic.
    pub direction: EventDirection,

    /// HTTP method (GET, POST, etc.).
    pub method: String,

    /// Full URL of the upstream request.
    pub upstream_target: String,

    /// Detected provider.
    pub provider: Provider,

    /// Model name (e.g. "gpt-4o", "claude-3-5-sonnet-20241022").
    pub model: Option<String>,

    // ── Payload ───────────────────────────────────────────────────────────────
    /// HTTP status code of the upstream response.
    pub status_code: Option<u16>,

    /// Reason the model stopped generating (e.g. "stop", "length", "tool_calls").
    pub finish_reason: Option<String>,

    /// Full request/response payload as JSONB. Stored compressed.
    pub payload: Option<JsonValue>,

    /// Raw wire size of the request+response in bytes.
    pub raw_size_bytes: Option<i64>,

    // ── Token & Cost metrics ──────────────────────────────────────────────────
    /// Prompt / input tokens consumed.
    pub input_tokens: Option<i32>,

    /// Completion / output tokens generated.
    pub output_tokens: Option<i32>,

    /// Total tokens (input + output).
    pub total_tokens: Option<i32>,

    /// Estimated USD cost for this event (DECIMAL 12,8 precision).
    pub cost_usd: Option<rust_decimal::Decimal>,

    // ── Governance ────────────────────────────────────────────────────────────
    /// PII findings (type + location only, NEVER actual PII values).
    pub pii_detected: Vec<PiiDetection>,

    /// Tool/function calls made during this event.
    pub tools_called: Vec<String>,

    /// Compliance: Merkle-chain SHA-256 hash linking this event to predecessor.
    /// First event in a session uses "GENESIS" seed.
    pub lineage_hash: String,

    /// Compliance: policy evaluation result ("pass:all", "warn:cost_budget", "audit:none", etc.).
    pub compliance_tag: String,

    /// Arbitrary tags as JSONB (e.g. {"env": "prod", "team": "ml"}).
    pub tags: JsonValue,

    /// Error message if the upstream request failed.
    pub error_message: Option<String>,

    // ── Audit ─────────────────────────────────────────────────────────────────
    /// When this record was inserted into the database.
    pub created_at: DateTime<Utc>,
}

impl AgentEvent {
    /// Create a new event with the four mandatory compliance fields pre-populated.
    ///
    /// Callers MUST provide `session_id` from `SessionTracker` (not random),
    /// `lineage_hash` from `compute_lineage_hash()`, and a valid `compliance_tag`.
    ///
    /// The 8 arguments are all required compliance-first fields — no subset is safe to omit.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        agent_id: impl Into<String>,
        session_id: Uuid,
        direction: EventDirection,
        method: impl Into<String>,
        upstream_target: impl Into<String>,
        provider: Provider,
        lineage_hash: impl Into<String>,
        compliance_tag: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            session_id,
            agent_id: agent_id.into(),
            timestamp: now,
            latency_ms: None,
            direction,
            method: method.into(),
            upstream_target: upstream_target.into(),
            provider,
            model: None,
            status_code: None,
            finish_reason: None,
            payload: None,
            raw_size_bytes: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            cost_usd: None,
            pii_detected: Vec::new(),
            tools_called: Vec::new(),
            lineage_hash: lineage_hash.into(),
            compliance_tag: compliance_tag.into(),
            tags: JsonValue::Object(Default::default()),
            error_message: None,
            created_at: now,
        }
    }
}

// Bring in rust_decimal only for the Decimal type; we re-export it for consumers.
mod decimal_import {
    // This module exists solely to document the rust_decimal dependency.
    // The Decimal type in AgentEvent comes from the rust_decimal crate.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_has_all_compliance_fields() {
        let session_id = Uuid::now_v7();
        let event = AgentEvent::new(
            "agent-001",
            session_id,
            EventDirection::Outbound,
            "POST",
            "https://api.openai.com/v1/chat/completions",
            Provider::OpenAI,
            "abc123lineagehash",
            "audit:none",
        );
        assert_eq!(event.session_id, session_id);
        assert!(!event.timestamp.to_rfc3339().is_empty());
        assert!(!event.lineage_hash.is_empty());
        assert!(!event.compliance_tag.is_empty());
    }

    #[test]
    fn event_serializes_to_json() {
        let session_id = Uuid::now_v7();
        let event = AgentEvent::new(
            "agent-002",
            session_id,
            EventDirection::Outbound,
            "POST",
            "https://api.anthropic.com/v1/messages",
            Provider::Anthropic,
            "genesishash",
            "pass:all",
        );
        let json = serde_json::to_string(&event).expect("serialization failed");
        assert!(json.contains("agent-002"));
        assert!(json.contains("anthropic"));
    }
}
