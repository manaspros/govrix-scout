//! Parse → log → forward pipeline.
//!
//! The interceptor builds AgentEvent objects and sends them to the event channel.
//! It runs ASYNCHRONOUSLY after the response is forwarded.
//! It MUST NOT block or slow the client.
//!
//! Compliance-first invariant (compliance-first skill):
//! Every intercepted action MUST produce: session_id, timestamp, lineage_hash, compliance_tag.
//!
//! All four compliance fields are set by this module:
//! - session_id: from SessionTracker (groups related requests)
//! - timestamp: set in AgentEvent::new() via Utc::now()
//! - lineage_hash: SHA-256 chain linking events (compute_lineage_hash)
//! - compliance_tag: "audit:none" by default (policy engine sets it in Phase 2)

use bytes::Bytes;
use chrono::Utc;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Mutex;

use govrix_scout_common::models::event::{AgentEvent, EventDirection, Provider};
use govrix_scout_common::protocols::Protocol;

use crate::events::{compute_lineage_hash, EventSender, Metrics, SessionTracker};
use crate::policy::PolicyHook;
use crate::proxy::streaming::SseAccumulator;
use crate::proxy::upstream::UpstreamUrls;

/// Shared interceptor state — session tracker and event sender.
///
/// Wrapped in Arc<Mutex<>> for safe concurrent access across request handlers.
pub struct InterceptorState {
    pub session_tracker: Mutex<SessionTracker>,
    pub event_sender: EventSender,
    /// Shared Prometheus-facing metrics counters.
    pub metrics: Arc<Metrics>,
    /// Policy hook — called after building each event to compute compliance_tag.
    pub policy_hook: Arc<dyn PolicyHook>,
    /// Configurable upstream base URLs for each provider.
    pub upstream_urls: Arc<UpstreamUrls>,
    /// Optional database pool for agent status look-ups (kill switch).
    ///
    /// When `None` (e.g. proxy started without a database), the kill switch
    /// check is skipped and the request is forwarded (fail-open).
    pub db_pool: Option<govrix_scout_store::StorePool>,
}

impl InterceptorState {
    pub fn new(
        event_sender: EventSender,
        metrics: Arc<Metrics>,
        policy_hook: Arc<dyn PolicyHook>,
    ) -> Self {
        Self {
            session_tracker: Mutex::new(SessionTracker::new()),
            event_sender,
            metrics,
            policy_hook,
            upstream_urls: Arc::new(UpstreamUrls::default()),
            db_pool: None,
        }
    }

    /// Create a new InterceptorState with custom upstream URLs.
    pub fn with_upstream_urls(
        event_sender: EventSender,
        metrics: Arc<Metrics>,
        policy_hook: Arc<dyn PolicyHook>,
        upstream_urls: UpstreamUrls,
    ) -> Self {
        Self {
            session_tracker: Mutex::new(SessionTracker::new()),
            event_sender,
            metrics,
            policy_hook,
            upstream_urls: Arc::new(upstream_urls),
            db_pool: None,
        }
    }

    /// Create a new InterceptorState with custom upstream URLs and a database pool.
    ///
    /// Use this constructor when a database connection is available so that the
    /// kill-switch (agent status) check is enforced on every proxied request.
    pub fn with_pool_and_upstream_urls(
        event_sender: EventSender,
        metrics: Arc<Metrics>,
        policy_hook: Arc<dyn PolicyHook>,
        upstream_urls: UpstreamUrls,
        db_pool: govrix_scout_store::StorePool,
    ) -> Self {
        Self {
            session_tracker: Mutex::new(SessionTracker::new()),
            event_sender,
            metrics,
            policy_hook,
            upstream_urls: Arc::new(upstream_urls),
            db_pool: Some(db_pool),
        }
    }
}

/// Context for a single proxied request-response cycle.
pub struct RequestContext {
    /// The agent making the request.
    pub agent_id: String,
    /// Detected protocol.
    pub protocol: Protocol,
    /// HTTP method.
    pub method: String,
    /// Full upstream URL.
    pub upstream_url: String,
    /// Request body bytes (for analysis).
    pub request_body: Bytes,
    /// When the request was received (for latency measurement).
    pub request_time: chrono::DateTime<Utc>,
}

/// Build and send a request event to the event channel.
///
/// This is the "outbound" side of the event — logged when a request is
/// intercepted and forwarded upstream.
pub async fn log_request_event(ctx: &RequestContext, state: &InterceptorState) {
    // Assign session_id and compute lineage hash
    let event_id = uuid::Uuid::now_v7();
    let (session_id, prev_hash) = {
        let mut tracker = state.session_tracker.lock().await;
        tracker.get_or_create(&ctx.agent_id, &event_id)
    };

    let timestamp_ms = ctx.request_time.timestamp_millis();
    let lineage_hash = compute_lineage_hash(&prev_hash, &event_id, &ctx.agent_id, timestamp_ms);

    // Build the compliance event
    let provider = protocol_to_provider(&ctx.protocol);
    let mut event = AgentEvent::new(
        ctx.agent_id.clone(),
        session_id,
        EventDirection::Outbound,
        ctx.method.clone(),
        ctx.upstream_url.clone(),
        provider,
        lineage_hash.clone(),
        "audit:none", // overridden below by policy hook
    );

    // Override the generated id with our pre-computed one
    event.id = event_id;

    // Parse request-specific fields
    match &ctx.protocol {
        Protocol::OpenAI { .. } => {
            if let Some(req) =
                govrix_scout_common::protocols::openai::parse_request(&ctx.request_body)
            {
                event.model = Some(req.model);
                // Count tools defined in the request
                if let Some(tools) = req.tools {
                    if !tools.is_empty() {
                        event.tags = serde_json::json!({
                            "tools_available": tools.len()
                        });
                    }
                }
            }
        }
        Protocol::Anthropic { .. } => {
            if let Some(req) =
                govrix_scout_common::protocols::anthropic::parse_request(&ctx.request_body)
            {
                event.model = Some(req.model);
            }
        }
        Protocol::Mcp { server, .. } => {
            if let Some(req) = govrix_scout_common::protocols::mcp::parse_request(&ctx.request_body)
            {
                let tool = govrix_scout_common::protocols::mcp::extract_tool_name(&req);
                event.tags = serde_json::json!({
                    "mcp_server": server,
                    "mcp_method": req.method,
                    "mcp_tool": tool
                });
            }
        }
        _ => {}
    }

    event.raw_size_bytes = Some(ctx.request_body.len() as i64);

    // ── Policy hook: compute compliance_tag ──────────────────────────────────
    event.compliance_tag = state.policy_hook.compliance_tag(&event);

    // Update session lineage
    {
        let mut tracker = state.session_tracker.lock().await;
        tracker.record_event(&ctx.agent_id, event_id, lineage_hash);
    }

    // Increment the Prometheus requests counter — one per intercepted request.
    state.metrics.requests_total.fetch_add(1, Ordering::Relaxed);

    // Fire-and-forget send
    state.event_sender.send(event);
}

/// Build and send a response event to the event channel.
///
/// This is the "inbound" side — logged after the upstream response is received.
pub async fn log_response_event(
    ctx: &RequestContext,
    state: &InterceptorState,
    status_code: u16,
    response_body: &Bytes,
    latency_ms: u32,
) {
    let event_id = uuid::Uuid::now_v7();
    let (session_id, prev_hash) = {
        let mut tracker = state.session_tracker.lock().await;
        tracker.get_or_create(&ctx.agent_id, &event_id)
    };

    let timestamp_ms = Utc::now().timestamp_millis();
    let lineage_hash = compute_lineage_hash(&prev_hash, &event_id, &ctx.agent_id, timestamp_ms);

    let provider = protocol_to_provider(&ctx.protocol);
    let mut event = AgentEvent::new(
        ctx.agent_id.clone(),
        session_id,
        EventDirection::Inbound,
        ctx.method.clone(),
        ctx.upstream_url.clone(),
        provider,
        lineage_hash.clone(),
        "audit:none",
    );

    event.id = event_id;
    event.status_code = Some(status_code);
    event.latency_ms = Some(latency_ms);
    event.raw_size_bytes = Some(response_body.len() as i64);

    // Parse response-specific fields
    match &ctx.protocol {
        Protocol::OpenAI { streaming, .. } => {
            if *streaming {
                // For streaming, body is the accumulated SSE chunks
                let data_lines = crate::proxy::streaming::parse_sse_data_lines(response_body);
                let acc =
                    govrix_scout_common::protocols::openai::parse_streaming_chunks(&data_lines);
                event.model = acc.model.or_else(|| {
                    govrix_scout_common::protocols::openai::parse_model(&ctx.request_body)
                });
                if let Some(usage) = acc.usage {
                    event.input_tokens = Some(usage.prompt_tokens);
                    event.output_tokens = Some(usage.completion_tokens);
                    event.total_tokens = Some(usage.total_tokens);
                }
                event.finish_reason = acc.finish_reason;
                event.tools_called = acc.tool_calls;
            } else if let Some(resp) =
                govrix_scout_common::protocols::openai::parse_response(response_body)
            {
                event.model = Some(resp.model);
                if let Some(usage) = resp.usage {
                    event.input_tokens = Some(usage.prompt_tokens);
                    event.output_tokens = Some(usage.completion_tokens);
                    event.total_tokens = Some(usage.total_tokens);
                }
                event.finish_reason = resp.finish_reason;
                event.tools_called = resp.tool_calls;
            }
        }
        Protocol::Anthropic { streaming, .. } => {
            if *streaming {
                // For streaming, response_body contains accumulated SSE bytes
                let mut acc_state = SseAccumulator::new();
                acc_state.process_anthropic_chunk(response_body);
                let streaming_acc =
                    govrix_scout_common::protocols::anthropic::accumulate_streaming_events(
                        &acc_state.anthropic_events,
                    );
                event.model = streaming_acc.model.clone().or_else(|| {
                    govrix_scout_common::protocols::anthropic::parse_model(&ctx.request_body)
                });
                if let Some(usage) = streaming_acc.usage() {
                    event.input_tokens = Some(usage.input_tokens);
                    event.output_tokens = Some(usage.output_tokens);
                    event.total_tokens = Some(usage.input_tokens + usage.output_tokens);
                }
                event.finish_reason = streaming_acc.stop_reason;
                event.tools_called = streaming_acc.tool_calls;
            } else if let Some(resp) =
                govrix_scout_common::protocols::anthropic::parse_response(response_body)
            {
                event.model = Some(resp.model);
                event.input_tokens = Some(resp.usage.input_tokens);
                event.output_tokens = Some(resp.usage.output_tokens);
                event.total_tokens = Some(resp.usage.input_tokens + resp.usage.output_tokens);
                event.finish_reason = resp.stop_reason;
                event.tools_called = resp.tool_calls;
            }
        }
        _ => {}
    }

    // Estimate cost from token usage
    if let (Some(input), Some(output)) = (event.input_tokens, event.output_tokens) {
        if let Some(ref model_name) = event.model {
            if let Some(pricing) = govrix_scout_common::models::pricing::lookup_pricing(model_name)
            {
                event.cost_usd = Some(pricing.estimate_cost(input, output));
            }
        }
    }

    // ── Policy hook: compute compliance_tag ──────────────────────────────────
    event.compliance_tag = state.policy_hook.compliance_tag(&event);

    // ── Budget: record actual usage so counters reflect live traffic ──────────
    let tokens = event.total_tokens.unwrap_or(0) as u64;
    let cost = event
        .cost_usd
        .and_then(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d))
        .unwrap_or(0.0);
    state
        .policy_hook
        .record_usage(&ctx.agent_id, tokens, cost, state.db_pool.clone());

    // Update session lineage
    {
        let mut tracker = state.session_tracker.lock().await;
        tracker.record_event(&ctx.agent_id, event_id, lineage_hash);
    }

    // Fire-and-forget send
    state.event_sender.send(event);
}

/// Analyze a request body (legacy tracing-only interface).
///
/// Kept for compatibility and testing. The full event pipeline uses
/// `log_request_event` and `log_response_event` instead.
#[allow(dead_code)]
pub async fn analyze_request(
    protocol: &Protocol,
    agent_id: &str,
    body: &Bytes,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if body.is_empty() {
        return Ok(());
    }

    match protocol {
        Protocol::OpenAI { .. } => {
            let model = govrix_scout_common::protocols::openai::parse_model(body);
            tracing::debug!(
                agent = agent_id,
                model = ?model,
                bytes = body.len(),
                "intercepted OpenAI request"
            );
        }
        Protocol::Anthropic { .. } => {
            let model = govrix_scout_common::protocols::anthropic::parse_model(body);
            tracing::debug!(
                agent = agent_id,
                model = ?model,
                bytes = body.len(),
                "intercepted Anthropic request"
            );
        }
        Protocol::Mcp { server, .. } => {
            if let Some(req) = govrix_scout_common::protocols::mcp::parse_request(body) {
                let tool = govrix_scout_common::protocols::mcp::extract_tool_name(&req);
                tracing::debug!(
                    agent = agent_id,
                    server = server,
                    method = req.method,
                    tool = ?tool,
                    "intercepted MCP request"
                );
            }
        }
        Protocol::A2A => {
            tracing::debug!(agent = agent_id, "intercepted A2A request");
        }
        Protocol::Unknown => {
            tracing::debug!(
                agent = agent_id,
                bytes = body.len(),
                "intercepted unknown protocol request"
            );
        }
    }

    Ok(())
}

/// Analyze a complete response body after it has been forwarded (legacy interface).
///
/// Kept for compatibility. The full event pipeline uses `log_response_event`.
#[allow(dead_code)]
pub async fn analyze_response(
    protocol: &Protocol,
    agent_id: &str,
    body: &Bytes,
    status_code: u16,
    latency_ms: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = (agent_id, body, status_code, latency_ms);

    match protocol {
        Protocol::OpenAI { .. } => {
            let _usage = govrix_scout_common::protocols::openai::parse_usage(body);
        }
        Protocol::Anthropic { .. } => {
            let _usage = govrix_scout_common::protocols::anthropic::parse_usage(body);
        }
        _ => {}
    }

    Ok(())
}

/// Map a Protocol to its Provider enum variant.
pub fn protocol_to_provider(protocol: &Protocol) -> Provider {
    match protocol {
        Protocol::OpenAI { .. } => Provider::OpenAI,
        Protocol::Anthropic { .. } => Provider::Anthropic,
        Protocol::Mcp { .. } => Provider::Mcp,
        Protocol::A2A => Provider::A2A,
        Protocol::Unknown => Provider::Unknown,
    }
}

/// Returns true when the JSON value from `get_agent` indicates the agent is blocked.
///
/// Centralises the status-check logic so it can be tested without a database.
pub fn agent_json_is_blocked(agent_json: &serde_json::Value) -> bool {
    agent_json
        .get("status")
        .and_then(|s| s.as_str())
        .map(|s| s == "blocked")
        .unwrap_or(false)
}

/// Build the HTTP 403 response returned when a blocked agent tries to make a request.
pub fn build_agent_blocked_response() -> hyper::Response<http_body_util::Full<bytes::Bytes>> {
    let body = bytes::Bytes::from_static(
        b"{\"error\":\"agent_blocked\",\"message\":\"This agent has been retired and is no longer permitted to make requests\"}",
    );
    hyper::Response::builder()
        .status(403)
        .header("content-type", "application/json")
        .body(http_body_util::Full::new(body))
        .expect("static 403 response must be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_to_provider_mapping() {
        assert_eq!(
            protocol_to_provider(&Protocol::OpenAI {
                version: "v1".to_string(),
                streaming: false
            }),
            Provider::OpenAI
        );
        assert_eq!(
            protocol_to_provider(&Protocol::Anthropic {
                version: "v1".to_string(),
                streaming: false
            }),
            Provider::Anthropic
        );
        assert_eq!(protocol_to_provider(&Protocol::Unknown), Provider::Unknown);
    }

    #[tokio::test]
    async fn analyze_request_does_not_fail_on_empty_body() {
        let proto = Protocol::OpenAI {
            version: "v1".to_string(),
            streaming: false,
        };
        let result = analyze_request(&proto, "agent-1", &Bytes::new()).await;
        assert!(result.is_ok());
    }

    // ── Kill switch unit tests ────────────────────────────────────────────────

    #[test]
    fn blocked_agent_json_is_detected() {
        let json = serde_json::json!({"id": "agent-1", "status": "blocked"});
        assert!(agent_json_is_blocked(&json));
    }

    #[test]
    fn active_agent_json_is_not_blocked() {
        let json = serde_json::json!({"id": "agent-1", "status": "active"});
        assert!(!agent_json_is_blocked(&json));
    }

    #[test]
    fn error_agent_json_is_not_blocked() {
        let json = serde_json::json!({"id": "agent-1", "status": "error"});
        assert!(!agent_json_is_blocked(&json));
    }

    #[test]
    fn missing_status_field_is_not_blocked() {
        let json = serde_json::json!({"id": "agent-1"});
        assert!(!agent_json_is_blocked(&json));
    }

    #[test]
    fn blocked_response_is_403_with_correct_body() {
        use http_body_util::BodyExt;

        let resp = build_agent_blocked_response();
        assert_eq!(resp.status().as_u16(), 403);

        // Verify Content-Type header
        let ct = resp.headers().get("content-type").unwrap();
        assert_eq!(ct, "application/json");

        // Verify body parses as JSON with the expected fields
        let body_bytes = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { resp.into_body().collect().await.unwrap().to_bytes() });
        let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(v["error"], "agent_blocked");
        assert!(v["message"].as_str().is_some());
    }
}
