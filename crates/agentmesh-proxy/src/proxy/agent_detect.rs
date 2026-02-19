//! Agent identity extraction.
//!
//! Resolution order (from MEMORY.md Proxy Architecture):
//! 1. `X-AgentMesh-Agent-Id` header  — explicit agent ID
//! 2. `Agent-Name` header            — human-readable name as ID
//! 3. API key mapping                 — lookup agent by Authorization key (stub)
//! 4. Source IP                       — IP address as fallback ID
//! 5. "unknown"                       — last resort

use std::net::SocketAddr;

/// Resolve the agent identity from HTTP headers and peer address.
///
/// Returns a stable string ID suitable for use as `agent_id` in events and registry.
pub fn resolve_agent_id(headers: &http::HeaderMap, peer_addr: SocketAddr) -> String {
    // 1. Explicit agent ID header
    if let Some(id) = headers
        .get("x-agentmesh-agent-id")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
    {
        return id.to_string();
    }

    // 2. Agent-Name header
    if let Some(name) = headers
        .get("agent-name")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
    {
        return name.to_string();
    }

    // 3. API key mapping (stub — Phase 1 will look up in agents table)
    if let Some(api_key_hint) = extract_api_key_hint(headers) {
        return api_key_hint;
    }

    // 4. Source IP
    let ip = peer_addr.ip().to_string();
    if ip != "127.0.0.1" && ip != "::1" {
        return format!("ip:{}", ip);
    }

    // 5. Unknown
    "unknown".to_string()
}

/// Extract a short identifying hint from the Authorization header.
///
/// Returns the last 8 characters of the bearer token as a hint.
/// NEVER returns the full API key.
fn extract_api_key_hint(headers: &http::HeaderMap) -> Option<String> {
    let auth = headers.get("authorization").and_then(|v| v.to_str().ok())?;

    let token = auth.strip_prefix("Bearer ").unwrap_or(auth);
    if token.len() >= 8 {
        let suffix = &token[token.len() - 8..];
        Some(format!("key:...{}", suffix))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn make_addr() -> SocketAddr {
        SocketAddr::from_str("10.0.0.1:12345").unwrap()
    }

    fn make_localhost() -> SocketAddr {
        SocketAddr::from_str("127.0.0.1:12345").unwrap()
    }

    #[test]
    fn resolves_explicit_header() {
        let mut headers = http::HeaderMap::new();
        headers.insert("x-agentmesh-agent-id", "my-agent-v2".parse().unwrap());
        assert_eq!(resolve_agent_id(&headers, make_addr()), "my-agent-v2");
    }

    #[test]
    fn resolves_agent_name_header() {
        let mut headers = http::HeaderMap::new();
        headers.insert("agent-name", "langchain-bot".parse().unwrap());
        assert_eq!(resolve_agent_id(&headers, make_addr()), "langchain-bot");
    }

    #[test]
    fn falls_back_to_ip() {
        let headers = http::HeaderMap::new();
        assert_eq!(resolve_agent_id(&headers, make_addr()), "ip:10.0.0.1");
    }

    #[test]
    fn falls_back_to_unknown_on_localhost() {
        let headers = http::HeaderMap::new();
        assert_eq!(resolve_agent_id(&headers, make_localhost()), "unknown");
    }
}
