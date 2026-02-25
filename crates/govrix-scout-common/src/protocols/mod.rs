//! Protocol detection types for Govrix Scout proxy routing.
//!
//! The proxy uses these types to:
//! 1. Route to the correct upstream URL
//! 2. Select the correct response parser
//! 3. Apply provider-specific SSE handling
//!
//! See rust-proxy skill for the `detect_protocol()` function signature.

pub mod anthropic;
pub mod mcp;
pub mod openai;

use serde::{Deserialize, Serialize};

/// MCP transport mechanisms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    /// Server-Sent Events transport.
    Sse,
    /// HTTP with streaming (MCP Streamable HTTP).
    StreamableHttp,
    /// Classic JSON-RPC over HTTP.
    JsonRpc,
}

/// Detected upstream protocol.
///
/// Returned by `detect_protocol()` in the proxy hot path.
/// This enum drives both routing and response parsing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Protocol {
    /// OpenAI-compatible API (including third-party providers using the OpenAI format).
    OpenAI {
        /// API version extracted from the path (e.g. "v1").
        version: String,
        /// Whether the request has `stream: true`.
        streaming: bool,
    },

    /// Anthropic Messages API.
    Anthropic {
        /// API version from the `anthropic-version` header.
        version: String,
        /// Whether the request has `stream: true`.
        streaming: bool,
    },

    /// Model Context Protocol.
    Mcp {
        /// The MCP server name from the path segment (e.g. "filesystem").
        server: String,
        /// Transport mechanism detected.
        transport: McpTransport,
    },

    /// Agent-to-Agent protocol.
    A2A,

    /// Custom / unknown upstream — pass through without protocol-specific parsing.
    Unknown,
}

impl Protocol {
    /// Returns true if this protocol uses SSE streaming.
    pub fn is_streaming(&self) -> bool {
        match self {
            Protocol::OpenAI { streaming, .. } => *streaming,
            Protocol::Anthropic { streaming, .. } => *streaming,
            Protocol::Mcp {
                transport: McpTransport::Sse,
                ..
            } => true,
            _ => false,
        }
    }

    /// Returns the provider string for event logging.
    pub fn provider_str(&self) -> &'static str {
        match self {
            Protocol::OpenAI { .. } => "openai",
            Protocol::Anthropic { .. } => "anthropic",
            Protocol::Mcp { .. } => "mcp",
            Protocol::A2A => "a2a",
            Protocol::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.provider_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_detection() {
        let openai_stream = Protocol::OpenAI {
            version: "v1".to_string(),
            streaming: true,
        };
        assert!(openai_stream.is_streaming());

        let openai_no_stream = Protocol::OpenAI {
            version: "v1".to_string(),
            streaming: false,
        };
        assert!(!openai_no_stream.is_streaming());
    }

    #[test]
    fn mcp_sse_is_streaming() {
        let mcp = Protocol::Mcp {
            server: "filesystem".to_string(),
            transport: McpTransport::Sse,
        };
        assert!(mcp.is_streaming());
    }
}
