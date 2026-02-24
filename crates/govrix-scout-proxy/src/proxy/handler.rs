//! Core proxy request handler.
//!
//! Entry point for every proxied request. Implements:
//! 1. Protocol detection (`detect_protocol`)
//! 2. Agent identity extraction
//! 3. Request body tee (read once, forward + analyze)
//! 4. Upstream forwarding (streaming or buffered)
//! 5. Response tee (streaming passthrough + async analysis)
//! 6. Event generation (fire-and-forget to bounded channel)
//!
//! CRITICAL: This is the hot path. All analysis is fire-and-forget.
//! The client MUST NOT be blocked by any internal operation.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Request, Response};

use govrix_scout_common::protocols::Protocol;

use super::{agent_detect, interceptor, upstream};
use crate::proxy::interceptor::{InterceptorState, RequestContext};

/// Core proxy request handler — entry point for every proxied HTTP request.
///
/// Signature matches `hyper::service::service_fn` expectations.
pub async fn proxy_handler(
    req: Request<Incoming>,
    peer_addr: SocketAddr,
    state: Arc<InterceptorState>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let start = Instant::now();
    let (parts, body) = req.into_parts();

    // ── Protocol detection (hot path, sync) ───────────────────────────────────
    let protocol = detect_protocol_from_parts(&parts);

    // ── Agent identity (hot path, fast) ───────────────────────────────────────
    let agent_id = agent_detect::resolve_agent_id(&parts.headers, peer_addr);

    // ── Request body tee ──────────────────────────────────────────────────────
    // Read the body once; clone bytes for analysis. For requests < 1MB this is fine.
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            tracing::warn!("failed to read request body: {}", e);
            // Fail-open: continue with empty body tee
            Bytes::new()
        }
    };

    // ── Refine streaming detection from body ──────────────────────────────────
    // The initial protocol detection only uses headers. Now we have the body,
    // so we can check `stream: true` in the request JSON.
    let protocol = refine_streaming_from_body(protocol, &body_bytes);
    let is_streaming = protocol.is_streaming();

    // ── Build upstream URL ────────────────────────────────────────────────────
    let upstream_url = upstream::build_upstream_url(&parts, &protocol, &state.upstream_urls);

    // ── Build request context for event logging ───────────────────────────────
    let ctx = Arc::new(RequestContext {
        agent_id: agent_id.clone(),
        protocol: protocol.clone(),
        method: parts.method.to_string(),
        upstream_url: upstream_url.clone(),
        request_body: body_bytes.clone(),
        request_time: chrono::Utc::now(),
    });

    // ── Fire-and-forget request event ─────────────────────────────────────────
    {
        let ctx_clone = Arc::clone(&ctx);
        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            interceptor::log_request_event(&ctx_clone, &state_clone).await;
        });
    }

    // ── Forward to upstream ───────────────────────────────────────────────────
    let response = if is_streaming {
        forward_streaming(parts, body_bytes, &protocol, ctx, state, start).await
    } else {
        forward_buffered(parts, body_bytes, &protocol, ctx, state, start).await
    };

    match response {
        Ok(resp) => Ok(resp),
        Err(e) => {
            tracing::error!("upstream forwarding error: {}", e);
            // Fail-open: return 502 rather than hanging
            let body = Bytes::from_static(b"{\"error\":\"upstream error\",\"code\":502}");
            Ok(Response::builder()
                .status(502)
                .header("content-type", "application/json")
                .body(Full::new(body))
                .unwrap())
        }
    }
}

/// Forward a non-streaming request — buffer full response, then log and return.
async fn forward_buffered(
    parts: http::request::Parts,
    body_bytes: Bytes,
    protocol: &Protocol,
    ctx: Arc<RequestContext>,
    state: Arc<InterceptorState>,
    start: Instant,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    let resp = upstream::forward(parts, body_bytes, protocol, &state.upstream_urls).await?;
    let latency_ms = start.elapsed().as_millis() as u32;

    let status = resp.status().as_u16();

    // Extract response body bytes for analysis
    // Full<Bytes> can be collected via BodyExt
    let (resp_parts, resp_body) = resp.into_parts();
    let resp_bytes = match resp_body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => Bytes::new(),
    };

    // ── Fire-and-forget response event ────────────────────────────────────────
    {
        let resp_bytes_clone = resp_bytes.clone();
        let ctx_clone = Arc::clone(&ctx);
        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            interceptor::log_response_event(
                &ctx_clone,
                &state_clone,
                status,
                &resp_bytes_clone,
                latency_ms,
            )
            .await;
        });
    }

    // Reconstruct and return the response
    let response = Response::from_parts(resp_parts, Full::new(resp_bytes));
    Ok(response)
}

/// Forward a streaming (SSE) request — pass chunks through immediately.
///
/// Implements the stream-through pattern:
/// 1. Send request to upstream
/// 2. Collect the full streaming response (for now — true streaming in Phase 1)
/// 3. Return the full body to the client
/// 4. Log the complete response event asynchronously
///
/// NOTE: Phase 0 implementation collects the full body, then returns it.
/// Phase 1 will use hyper's streaming body to pass chunks through in real-time
/// using a tokio channel + Body impl. The analysis/logging path is already correct.
async fn forward_streaming(
    parts: http::request::Parts,
    body_bytes: Bytes,
    protocol: &Protocol,
    ctx: Arc<RequestContext>,
    state: Arc<InterceptorState>,
    start: Instant,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    let (status, content_type, resp_bytes) =
        upstream::forward_streaming_collect(parts, body_bytes, protocol, &state.upstream_urls)
            .await?;
    let latency_ms = start.elapsed().as_millis() as u32;

    tracing::debug!(
        agent = %ctx.agent_id,
        status,
        latency_ms,
        bytes = resp_bytes.len(),
        "streaming response collected"
    );

    // ── Process accumulated stream for logging ────────────────────────────────
    let accumulated_for_log = build_accumulated_stream_body(protocol, &resp_bytes);

    // ── Fire-and-forget response event ────────────────────────────────────────
    {
        let ctx_clone = Arc::clone(&ctx);
        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            interceptor::log_response_event(
                &ctx_clone,
                &state_clone,
                status,
                &accumulated_for_log,
                latency_ms,
            )
            .await;
        });
    }

    // Return the full response to client
    let response = Response::builder()
        .status(status)
        .header("content-type", content_type)
        .body(Full::new(resp_bytes))
        .unwrap();

    Ok(response)
}

/// For streaming responses, build the bytes to pass to the response logger.
///
/// For non-streaming protocols, returns the raw bytes as-is.
/// For SSE responses, returns all `data:` lines concatenated for analysis.
fn build_accumulated_stream_body(protocol: &Protocol, raw_bytes: &Bytes) -> Bytes {
    match protocol {
        Protocol::OpenAI {
            streaming: true, ..
        } => {
            // The interceptor's parse_response path handles both streaming and buffered
            // For streaming, pass the raw SSE bytes — parse_streaming_chunks is called there
            raw_bytes.clone()
        }
        Protocol::Anthropic {
            streaming: true, ..
        } => {
            // Pass raw SSE bytes — process_anthropic_chunk is called in log_response_event
            raw_bytes.clone()
        }
        _ => raw_bytes.clone(),
    }
}

/// Detect the upstream protocol from request parts.
///
/// Matches the `detect_protocol()` signature from the rust-proxy skill.
fn detect_protocol_from_parts(parts: &http::request::Parts) -> Protocol {
    let path = parts.uri.path();
    let headers = &parts.headers;

    if path.starts_with("/proxy/openai/") {
        Protocol::OpenAI {
            version: extract_api_version(path, "openai"),
            streaming: is_streaming_hint_from_headers(headers),
        }
    } else if path.starts_with("/proxy/anthropic/") {
        Protocol::Anthropic {
            version: extract_api_version_anthropic(parts),
            streaming: is_streaming_hint_from_headers(headers),
        }
    } else if path.starts_with("/proxy/mcp/") {
        Protocol::Mcp {
            server: extract_mcp_server(path),
            transport: detect_mcp_transport(headers),
        }
    } else if path.starts_with("/proxy/a2a/") || is_a2a_request(headers) {
        Protocol::A2A
    } else {
        Protocol::Unknown
    }
}

/// Refine the streaming flag after reading the body.
///
/// The body may contain `"stream": true` which overrides the header hint.
fn refine_streaming_from_body(protocol: Protocol, body: &Bytes) -> Protocol {
    match protocol {
        Protocol::OpenAI { version, .. } => {
            let streaming = govrix_scout_common::protocols::openai::is_streaming(body);
            Protocol::OpenAI { version, streaming }
        }
        Protocol::Anthropic { version, .. } => {
            let streaming = govrix_scout_common::protocols::anthropic::is_streaming(body);
            Protocol::Anthropic { version, streaming }
        }
        other => other,
    }
}

fn extract_api_version(path: &str, provider: &str) -> String {
    // e.g. "/proxy/openai/v1/chat/completions" → "v1"
    let prefix = format!("/proxy/{}/", provider);
    let rest = path.strip_prefix(&prefix).unwrap_or(path);
    rest.split('/').next().unwrap_or("v1").to_string()
}

fn extract_api_version_anthropic(parts: &http::request::Parts) -> String {
    // Prefer anthropic-version header, fall back to path-based extraction
    parts
        .headers
        .get("anthropic-version")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| extract_api_version(parts.uri.path(), "anthropic"))
}

fn extract_mcp_server(path: &str) -> String {
    // e.g. "/proxy/mcp/filesystem/..." → "filesystem"
    let rest = path.strip_prefix("/proxy/mcp/").unwrap_or("");
    rest.split('/').next().unwrap_or("unknown").to_string()
}

fn is_streaming_hint_from_headers(headers: &http::HeaderMap) -> bool {
    // Check Accept header as an early hint — refined after body read
    headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/event-stream"))
        .unwrap_or(false)
}

fn detect_mcp_transport(headers: &http::HeaderMap) -> govrix_scout_common::protocols::McpTransport {
    if headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/event-stream"))
        .unwrap_or(false)
    {
        govrix_scout_common::protocols::McpTransport::Sse
    } else {
        govrix_scout_common::protocols::McpTransport::JsonRpc
    }
}

fn is_a2a_request(headers: &http::HeaderMap) -> bool {
    headers.contains_key("x-a2a-protocol-version")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_protocol_openai_path() {
        let req = http::Request::builder()
            .method("POST")
            .uri("/proxy/openai/v1/chat/completions")
            .body(())
            .unwrap();
        let (parts, _) = req.into_parts();
        let proto = detect_protocol_from_parts(&parts);
        assert!(matches!(proto, Protocol::OpenAI { .. }));
    }

    #[test]
    fn detect_protocol_anthropic_path() {
        let req = http::Request::builder()
            .method("POST")
            .uri("/proxy/anthropic/v1/messages")
            .body(())
            .unwrap();
        let (parts, _) = req.into_parts();
        let proto = detect_protocol_from_parts(&parts);
        assert!(matches!(proto, Protocol::Anthropic { .. }));
    }

    #[test]
    fn detect_protocol_unknown_path() {
        let req = http::Request::builder()
            .method("GET")
            .uri("/health")
            .body(())
            .unwrap();
        let (parts, _) = req.into_parts();
        let proto = detect_protocol_from_parts(&parts);
        assert_eq!(proto, Protocol::Unknown);
    }

    #[test]
    fn refine_streaming_from_body_openai() {
        let proto = Protocol::OpenAI {
            version: "v1".to_string(),
            streaming: false,
        };
        let body = Bytes::from(br#"{"model":"gpt-4o","stream":true,"messages":[]}"#.to_vec());
        let refined = refine_streaming_from_body(proto, &body);
        assert!(matches!(
            refined,
            Protocol::OpenAI {
                streaming: true,
                ..
            }
        ));
    }

    #[test]
    fn refine_streaming_from_body_no_stream() {
        let proto = Protocol::OpenAI {
            version: "v1".to_string(),
            streaming: false,
        };
        let body = Bytes::from(br#"{"model":"gpt-4o","messages":[]}"#.to_vec());
        let refined = refine_streaming_from_body(proto, &body);
        assert!(matches!(
            refined,
            Protocol::OpenAI {
                streaming: false,
                ..
            }
        ));
    }

    #[test]
    fn extract_api_version_from_path() {
        assert_eq!(
            extract_api_version("/proxy/openai/v1/chat/completions", "openai"),
            "v1"
        );
        assert_eq!(
            extract_api_version("/proxy/openai/v2/chat/completions", "openai"),
            "v2"
        );
    }

    #[test]
    fn extract_mcp_server_from_path() {
        assert_eq!(
            extract_mcp_server("/proxy/mcp/filesystem/tools/call"),
            "filesystem"
        );
        assert_eq!(extract_mcp_server("/proxy/mcp/postgres/query"), "postgres");
    }
}
