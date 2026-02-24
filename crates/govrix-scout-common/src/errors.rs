//! Unified error types for Govrix Scout.

use thiserror::Error;

/// Top-level error type for Govrix Scout operations.
#[derive(Debug, Error)]
pub enum Govrix ScoutError {
    #[error("database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("proxy error: {0}")]
    Proxy(#[from] ProxyError),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

/// Database-specific errors.
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("connection failed: {0}")]
    Connection(String),

    #[error("query failed: {0}")]
    Query(String),

    #[error("record not found: {kind} with id={id}")]
    NotFound { kind: String, id: String },

    #[error("constraint violation: {0}")]
    Constraint(String),

    #[error("migration failed: {0}")]
    Migration(String),
}

/// Configuration-specific errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file at {path}: {source}")]
    FileRead {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to parse TOML config: {0}")]
    ParseToml(#[from] toml::de::Error),

    #[error("missing required config key: {0}")]
    MissingKey(String),

    #[error("invalid config value for '{key}': {reason}")]
    InvalidValue { key: String, reason: String },
}

/// Proxy-specific errors.
#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("upstream connection failed: {0}")]
    UpstreamConnect(String),

    #[error("upstream request failed with status {status}: {body}")]
    UpstreamStatus { status: u16, body: String },

    #[error("failed to read request body: {0}")]
    RequestBody(String),

    #[error("failed to read response body: {0}")]
    ResponseBody(String),

    #[error("protocol detection failed: {0}")]
    ProtocolDetect(String),

    #[error("SSE streaming error: {0}")]
    SseStream(String),

    #[error("agent identity could not be resolved")]
    AgentUnresolved,
}

impl Govrix ScoutError {
    /// Create an "other" error from any displayable value.
    pub fn other(msg: impl std::fmt::Display) -> Self {
        Self::Other(msg.to_string())
    }
}
