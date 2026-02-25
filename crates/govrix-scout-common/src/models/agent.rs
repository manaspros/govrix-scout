//! Agent — registered AI agent identity and runtime statistics.
//!
//! Schema matches the `agents` table from MEMORY.md:
//!   id(VARCHAR PK), name, description, framework, status, labels(JSONB),
//!   total_requests, total_tokens, total_cost_usd, last_model_used,
//!   last_seen_at, last_error_at, error_count, source_ip(INET),
//!   first_seen_at, created_at, updated_at

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Agent framework classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    McpClient,
    Langchain,
    Crewai,
    Autogen,
    DirectApi,
    A2A,
    Custom,
    Unknown,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AgentType::McpClient => "mcp_client",
            AgentType::Langchain => "langchain",
            AgentType::Crewai => "crewai",
            AgentType::Autogen => "autogen",
            AgentType::DirectApi => "direct_api",
            AgentType::A2A => "a2a",
            AgentType::Custom => "custom",
            AgentType::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AgentStatus::Active => "active",
            AgentStatus::Idle => "idle",
            AgentStatus::Error => "error",
            AgentStatus::Blocked => "blocked",
        };
        write!(f, "{}", s)
    }
}

/// Agent lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    #[default]
    Active,
    Idle,
    Error,
    Blocked,
}

/// An AI agent registered in the Govrix Scout registry.
///
/// Maps to the `agents` table. The `id` is a VARCHAR primary key — typically
/// the agent identifier extracted from the `X-govrix-scout-Agent-Id` header,
/// the `Agent-Name` header, API key mapping, or source IP fallback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    // ── Identity ──────────────────────────────────────────────────────────────
    /// Primary key — agent identifier string (not UUID, see schema).
    pub id: String,

    /// Human-readable display name.
    pub name: Option<String>,

    /// Optional description of the agent's purpose.
    pub description: Option<String>,

    /// The framework or type of this agent.
    pub agent_type: AgentType,

    // ── Lifecycle ─────────────────────────────────────────────────────────────
    /// Current operational status.
    pub status: AgentStatus,

    /// When this agent was first observed by the proxy.
    pub first_seen_at: DateTime<Utc>,

    /// When this agent last made a request.
    pub last_seen_at: DateTime<Utc>,

    /// When the last error was recorded for this agent.
    pub last_error_at: Option<DateTime<Utc>>,

    // ── Network ───────────────────────────────────────────────────────────────
    /// Source IP address of the agent (stored as text; INET in PostgreSQL).
    pub source_ip: Option<String>,

    /// Composite fingerprint hash for identifying agents without explicit ID.
    pub fingerprint: Option<String>,

    // ── API targets ───────────────────────────────────────────────────────────
    /// LLM API endpoints this agent calls (JSONB array of URL strings).
    pub target_apis: JsonValue,

    /// MCP servers this agent connects to (JSONB array of server names/URLs).
    pub mcp_servers: JsonValue,

    // ── Aggregate statistics ──────────────────────────────────────────────────
    /// Total number of proxy requests from this agent.
    pub total_requests: i64,

    /// Total input tokens consumed across all requests.
    pub total_tokens_in: i64,

    /// Total output tokens generated across all requests.
    pub total_tokens_out: i64,

    /// Total estimated USD cost for this agent.
    pub total_cost_usd: rust_decimal::Decimal,

    /// The most recently used model name.
    pub last_model_used: Option<String>,

    /// Number of errors recorded for this agent.
    pub error_count: i64,

    // ── Labels & metadata ─────────────────────────────────────────────────────
    /// Arbitrary labels for filtering and grouping (JSONB).
    pub labels: JsonValue,

    /// Governance policy IDs assigned to this agent.
    pub policy_ids: Vec<uuid::Uuid>,

    /// Arbitrary metadata (JSONB).
    pub metadata: JsonValue,

    // ── Audit timestamps ──────────────────────────────────────────────────────
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Agent {
    /// Create a new agent with minimal required fields.
    pub fn new(id: impl Into<String>, agent_type: AgentType) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: None,
            description: None,
            agent_type,
            status: AgentStatus::Active,
            first_seen_at: now,
            last_seen_at: now,
            last_error_at: None,
            source_ip: None,
            fingerprint: None,
            target_apis: JsonValue::Array(Vec::new()),
            mcp_servers: JsonValue::Array(Vec::new()),
            total_requests: 0,
            total_tokens_in: 0,
            total_tokens_out: 0,
            total_cost_usd: rust_decimal::Decimal::ZERO,
            last_model_used: None,
            error_count: 0,
            labels: JsonValue::Object(Default::default()),
            policy_ids: Vec::new(),
            metadata: JsonValue::Object(Default::default()),
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_default_status_is_active() {
        let agent = Agent::new("test-agent", AgentType::DirectApi);
        assert_eq!(agent.status, AgentStatus::Active);
        assert_eq!(agent.total_requests, 0);
    }

    #[test]
    fn agent_type_display() {
        assert_eq!(AgentType::McpClient.to_string(), "mcp_client");
        assert_eq!(AgentType::Langchain.to_string(), "langchain");
    }
}
