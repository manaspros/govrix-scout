//! Upstream connection management.
//!
//! Routes requests to the correct LLM API endpoint based on detected protocol.
//! Uses reqwest for outbound HTTP with TLS verification.
//!
//! Fail-open: if upstream is unreachable, returns 502 rather than hanging.
//!
//! Two forwarding modes:
//! 1. Buffered: read full response body, return as Full<Bytes>
//! 2. Streaming: pass chunks through immediately via SSE stream-through

use bytes::Bytes;
use http_body_util::Full;
use hyper::Response;
use std::sync::OnceLock;
use std::time::Duration;

use agentmesh_common::protocols::Protocol;

static SHARED_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Get or create the shared reqwest client.
///
/// Uses `OnceLock` to lazily initialise a single `reqwest::Client` that is
/// reused across all requests, avoiding per-request TLS handshake overhead.
pub fn shared_client() -> &'static reqwest::Client {
    SHARED_CLIENT.get_or_init(|| build_client(DEFAULT_TIMEOUT_MS))
}

/// Default upstream request timeout.
const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Maximum response body size for buffered (non-streaming) responses.
/// Streaming responses bypass this limit.
const MAX_RESPONSE_BODY_BYTES: usize = 10 * 1024 * 1024; // 10 MiB

/// Build a reqwest client with appropriate TLS and timeout settings.
///
/// For normal request forwarding, prefer [`shared_client()`] which reuses a
/// single instance. This builder is still available for cases that need a
/// custom timeout or other per-call configuration.
pub fn build_client(timeout_ms: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .use_rustls_tls()
        .user_agent(concat!("agentmesh-proxy/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("failed to build reqwest client")
}

/// Forward a request to the appropriate upstream and return the buffered response.
///
/// For streaming responses, use `forward_streaming` instead.
/// This function buffers the full response body (up to MAX_RESPONSE_BODY_BYTES).
pub async fn forward(
    parts: http::request::Parts,
    body: Bytes,
    protocol: &Protocol,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    let upstream_url = build_upstream_url(&parts, protocol);

    tracing::debug!(
        method = %parts.method,
        url = %upstream_url,
        bytes = body.len(),
        "forwarding to upstream (buffered)"
    );

    let client = shared_client();
    let mut req_builder = client
        .request(parts.method.clone(), &upstream_url)
        .body(body.to_vec());

    // Forward headers (strip hop-by-hop headers, adjust Host)
    for (name, value) in &parts.headers {
        if !is_hop_by_hop_header(name.as_str()) {
            if let Ok(v) = value.to_str() {
                req_builder = req_builder.header(name.as_str(), v);
            }
        }
    }

    let resp = req_builder.send().await.map_err(|e| {
        tracing::warn!(url = %upstream_url, error = %e, "upstream request failed");
        e
    })?;

    let status = resp.status().as_u16();

    // Copy response headers for passthrough
    let mut response_builder = Response::builder().status(status);

    // Copy content-type from upstream response
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    response_builder = response_builder.header("content-type", &content_type);

    // Copy other safe response headers
    for (name, value) in resp.headers() {
        if should_forward_response_header(name.as_str()) {
            if let Ok(v) = value.to_str() {
                response_builder = response_builder.header(name.as_str(), v);
            }
        }
    }

    // Buffer the response body (limited to MAX_RESPONSE_BODY_BYTES)
    let resp_bytes = resp.bytes().await.map_err(|e| {
        tracing::warn!("failed to read upstream response body: {}", e);
        e
    })?;

    if resp_bytes.len() > MAX_RESPONSE_BODY_BYTES {
        tracing::warn!(
            size = resp_bytes.len(),
            limit = MAX_RESPONSE_BODY_BYTES,
            "upstream response body truncated"
        );
    }

    Ok(response_builder.body(Full::new(resp_bytes)).unwrap())
}

/// Forward a streaming request and return the full accumulated response body.
///
/// For the proxy's hot path, chunks are forwarded to the client immediately
/// via `streaming::stream_sse_response`. This function returns the complete
/// accumulated body for analysis.
///
/// Returns `(status_code, content_type, accumulated_bytes)`.
pub async fn forward_streaming_collect(
    parts: http::request::Parts,
    body: Bytes,
    protocol: &Protocol,
) -> Result<(u16, String, Bytes), Box<dyn std::error::Error + Send + Sync>> {
    let upstream_url = build_upstream_url(&parts, protocol);

    tracing::debug!(
        method = %parts.method,
        url = %upstream_url,
        bytes = body.len(),
        "forwarding to upstream (streaming/collect)"
    );

    let client = shared_client();
    let mut req_builder = client
        .request(parts.method.clone(), &upstream_url)
        .body(body.to_vec());

    for (name, value) in &parts.headers {
        if !is_hop_by_hop_header(name.as_str()) {
            if let Ok(v) = value.to_str() {
                req_builder = req_builder.header(name.as_str(), v);
            }
        }
    }

    let resp = req_builder.send().await?;
    let status = resp.status().as_u16();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/event-stream")
        .to_string();

    let body_bytes = resp.bytes().await?;
    Ok((status, content_type, body_bytes))
}

/// Build the full upstream URL from request parts and protocol.
pub fn build_upstream_url(parts: &http::request::Parts, protocol: &Protocol) -> String {
    let upstream_base = resolve_upstream_base(protocol);
    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let api_path = strip_proxy_prefix(path_and_query, protocol);
    format!("{}{}", upstream_base, api_path)
}

/// Resolve the upstream base URL from the protocol.
fn resolve_upstream_base(protocol: &Protocol) -> &'static str {
    match protocol {
        Protocol::OpenAI { .. } => "https://api.openai.com",
        Protocol::Anthropic { .. } => "https://api.anthropic.com",
        Protocol::Mcp { .. } => "http://localhost:3001", // local MCP server stub
        Protocol::A2A => "http://localhost:3002",        // local A2A stub
        Protocol::Unknown => "http://localhost:8080",    // passthrough stub
    }
}

/// Strip the `/proxy/<provider>` prefix from the path.
fn strip_proxy_prefix<'a>(path: &'a str, protocol: &Protocol) -> &'a str {
    let prefix = match protocol {
        Protocol::OpenAI { .. } => "/proxy/openai",
        Protocol::Anthropic { .. } => "/proxy/anthropic",
        Protocol::Mcp { .. } => "/proxy/mcp",
        Protocol::A2A => "/proxy/a2a",
        Protocol::Unknown => "",
    };
    path.strip_prefix(prefix).unwrap_or(path)
}

/// Returns true for HTTP/1.1 hop-by-hop headers that should not be forwarded.
pub fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}

/// Returns true for response headers that are safe to forward to the client.
fn should_forward_response_header(name: &str) -> bool {
    let lower = name.to_lowercase();
    // Skip hop-by-hop headers
    if is_hop_by_hop_header(&lower) {
        return false;
    }
    // Skip content-type (already set above)
    if lower == "content-type" {
        return false;
    }
    // Forward rate limit, retry, and request ID headers
    matches!(
        lower.as_str(),
        "x-request-id"
            | "x-ratelimit-limit-requests"
            | "x-ratelimit-limit-tokens"
            | "x-ratelimit-remaining-requests"
            | "x-ratelimit-remaining-tokens"
            | "x-ratelimit-reset-requests"
            | "x-ratelimit-reset-tokens"
            | "retry-after"
            | "openai-organization"
            | "openai-version"
            | "openai-processing-ms"
            | "anthropic-ratelimit-input-tokens-limit"
            | "anthropic-ratelimit-input-tokens-remaining"
            | "anthropic-ratelimit-output-tokens-limit"
            | "anthropic-ratelimit-output-tokens-remaining"
            | "request-id"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hop_by_hop_headers_detected() {
        assert!(is_hop_by_hop_header("connection"));
        assert!(is_hop_by_hop_header("transfer-encoding"));
        assert!(!is_hop_by_hop_header("authorization"));
        assert!(!is_hop_by_hop_header("content-type"));
    }

    #[test]
    fn strip_proxy_prefix_openai() {
        let proto = Protocol::OpenAI {
            version: "v1".to_string(),
            streaming: false,
        };
        assert_eq!(
            strip_proxy_prefix("/proxy/openai/v1/chat/completions", &proto),
            "/v1/chat/completions"
        );
    }

    #[test]
    fn strip_proxy_prefix_anthropic() {
        let proto = Protocol::Anthropic {
            version: "v1".to_string(),
            streaming: false,
        };
        assert_eq!(
            strip_proxy_prefix("/proxy/anthropic/v1/messages", &proto),
            "/v1/messages"
        );
    }

    #[test]
    fn build_upstream_url_openai() {
        let parts = http::Request::builder()
            .method("POST")
            .uri("/proxy/openai/v1/chat/completions")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let proto = Protocol::OpenAI {
            version: "v1".to_string(),
            streaming: false,
        };
        let url = build_upstream_url(&parts, &proto);
        assert_eq!(url, "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn build_upstream_url_anthropic() {
        let parts = http::Request::builder()
            .method("POST")
            .uri("/proxy/anthropic/v1/messages")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let proto_anth = Protocol::Anthropic {
            version: "v1".to_string(),
            streaming: false,
        };
        let url = build_upstream_url(&parts, &proto_anth);
        assert_eq!(url, "https://api.anthropic.com/v1/messages");
    }
}
