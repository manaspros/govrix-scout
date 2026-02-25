//! Model Context Protocol (MCP) parser stub.
//!
//! Handles JSON-RPC 2.0 messages over HTTP/SSE.
//! MCP tool calls are the primary "action" type to log for governance.
//!
//! Transport types: SSE, Streamable HTTP, JSON-RPC

use serde::{Deserialize, Serialize};

/// MCP JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// MCP JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

/// MCP JSON-RPC error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Well-known MCP methods.
pub mod methods {
    pub const INITIALIZE: &str = "initialize";
    pub const TOOLS_LIST: &str = "tools/list";
    pub const TOOLS_CALL: &str = "tools/call";
    pub const RESOURCES_LIST: &str = "resources/list";
    pub const RESOURCES_READ: &str = "resources/read";
    pub const PROMPTS_LIST: &str = "prompts/list";
    pub const PROMPTS_GET: &str = "prompts/get";
    pub const SAMPLING_CREATE: &str = "sampling/createMessage";
}

/// Parse an MCP request from the request body bytes.
pub fn parse_request(body: &[u8]) -> Option<McpRequest> {
    serde_json::from_slice(body).ok()
}

/// Extract the tool name from a `tools/call` request.
///
/// Returns `None` if not a tool call or missing the name parameter.
pub fn extract_tool_name(req: &McpRequest) -> Option<String> {
    if req.method != methods::TOOLS_CALL {
        return None;
    }
    req.params.as_ref()?.get("name")?.as_str().map(String::from)
}

/// Check if this is a tool call method.
pub fn is_tool_call(method: &str) -> bool {
    method == methods::TOOLS_CALL
}
