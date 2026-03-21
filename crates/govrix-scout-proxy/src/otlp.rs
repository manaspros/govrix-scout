//! OTLP span export — publishes Govrix events as OpenTelemetry spans.
//!
//! This runs as a background task, reading from the batch after each flush
//! (NOT on the hot path). It converts AgentEvent records to OTLP/HTTP JSON
//! format and POSTs them to the configured endpoint.
//!
//! OpenTelemetry GenAI Semantic Conventions:
//! https://opentelemetry.io/docs/specs/semconv/gen-ai/

use govrix_scout_common::models::event::{AgentEvent, EventKind};
use serde_json::{json, Value};
use uuid::Uuid;

const OTLP_SCHEMA_URL: &str = "https://opentelemetry.io/schemas/1.26.0";

/// Convert an AgentEvent to an OTLP span JSON object.
pub fn event_to_otlp_span(event: &AgentEvent) -> Value {
    let trace_id_hex = event
        .trace_id
        .map(|u| format!("{}", u.as_simple()))
        .unwrap_or_else(|| format!("{}", Uuid::now_v7().as_simple()));

    // OTel span_id is 8 bytes (16 hex chars) — take first 16 chars of UUID simple form.
    let span_id_hex = {
        let full = format!("{}", event.span_id.as_simple());
        full[..16].to_string()
    };

    let parent_span_id_hex = event.parent_span_id.map(|u| {
        let full = format!("{}", u.as_simple());
        full[..16].to_string()
    });

    let start_nanos = event.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64;
    let end_nanos = start_nanos + (event.latency_ms.unwrap_or(0) as u64 * 1_000_000);

    // Map EventKind to OTel span name using GenAI semantic conventions.
    let span_name = match event.event_kind {
        EventKind::LlmRequest | EventKind::LlmResponse => {
            format!("gen_ai.{}", event.model.as_deref().unwrap_or("unknown"))
        }
        EventKind::ToolInvoke | EventKind::ToolResult => {
            format!("tool.{}", event.tool_name.as_deref().unwrap_or("unknown"))
        }
        _ => event.event_kind.to_string(),
    };

    let mut attributes = vec![
        json!({
            "key": "gen_ai.system",
            "value": {"stringValue": event.provider.to_string()}
        }),
        json!({
            "key": "gen_ai.request.model",
            "value": {"stringValue": event.model.as_deref().unwrap_or("")}
        }),
        json!({
            "key": "govrix.agent.id",
            "value": {"stringValue": event.agent_id}
        }),
        json!({
            "key": "govrix.session.id",
            "value": {"stringValue": event.session_id.to_string()}
        }),
        json!({
            "key": "govrix.event.kind",
            "value": {"stringValue": event.event_kind.to_string()}
        }),
        json!({
            "key": "govrix.compliance_tag",
            "value": {"stringValue": event.compliance_tag}
        }),
    ];

    if let Some(tokens) = event.input_tokens {
        attributes.push(json!({
            "key": "gen_ai.usage.input_tokens",
            "value": {"intValue": tokens}
        }));
    }
    if let Some(tokens) = event.output_tokens {
        attributes.push(json!({
            "key": "gen_ai.usage.output_tokens",
            "value": {"intValue": tokens}
        }));
    }
    if let Some(cost) = event.cost_usd {
        attributes.push(json!({
            "key": "govrix.cost_usd",
            "value": {"doubleValue": cost.to_string().parse::<f64>().unwrap_or(0.0)}
        }));
    }
    if let Some(tool) = &event.tool_name {
        attributes.push(json!({
            "key": "gen_ai.tool.name",
            "value": {"stringValue": tool}
        }));
    }
    if let Some(risk) = event.risk_score {
        attributes.push(json!({
            "key": "govrix.risk_score",
            "value": {"doubleValue": risk}
        }));
    }
    if let Some(latency) = event.latency_ms {
        attributes.push(json!({
            "key": "govrix.latency_ms",
            "value": {"intValue": latency}
        }));
    }
    if let Some(status) = event.status_code {
        attributes.push(json!({
            "key": "http.status_code",
            "value": {"intValue": status}
        }));
    }

    // OTel status: 1 = OK, 2 = ERROR
    let status_code = if event.status_code.is_some_and(|c| c >= 400) {
        2
    } else {
        1
    };

    let mut span = json!({
        "traceId": trace_id_hex,
        "spanId": span_id_hex,
        "name": span_name,
        "kind": 3, // SPAN_KIND_CLIENT
        "startTimeUnixNano": start_nanos.to_string(),
        "endTimeUnixNano": end_nanos.to_string(),
        "attributes": attributes,
        "status": {
            "code": status_code
        }
    });

    if let Some(parent_id) = parent_span_id_hex {
        span["parentSpanId"] = json!(parent_id);
    }

    span
}

/// Build an OTLP ExportTraceServiceRequest JSON body from a batch of events.
pub fn events_to_otlp_request(events: &[AgentEvent], service_name: &str) -> Value {
    let spans: Vec<Value> = events.iter().map(event_to_otlp_span).collect();

    json!({
        "resourceSpans": [{
            "resource": {
                "attributes": [
                    {
                        "key": "service.name",
                        "value": {"stringValue": service_name}
                    },
                    {
                        "key": "service.version",
                        "value": {"stringValue": env!("CARGO_PKG_VERSION")}
                    },
                    {
                        "key": "telemetry.sdk.name",
                        "value": {"stringValue": "govrix-scout"}
                    }
                ]
            },
            "schemaUrl": OTLP_SCHEMA_URL,
            "scopeSpans": [{
                "scope": {"name": "govrix-scout-proxy"},
                "spans": spans
            }]
        }]
    })
}

/// Export a batch of events to an OTLP/HTTP endpoint.
///
/// Fire-and-forget: errors are logged but not propagated.
/// Returns `Ok(())` immediately if `endpoint` is empty or `events` is empty.
pub async fn export_to_otlp(
    client: &reqwest::Client,
    endpoint: &str,
    events: &[AgentEvent],
    service_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if events.is_empty() || endpoint.is_empty() {
        return Ok(());
    }

    let body = events_to_otlp_request(events, service_name);

    let response = client
        .post(endpoint)
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;

    if !response.status().is_success() {
        tracing::warn!(
            status = %response.status(),
            endpoint = %endpoint,
            "OTLP export returned non-success status"
        );
    } else {
        tracing::debug!(
            event_count = events.len(),
            endpoint = %endpoint,
            "OTLP spans exported"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use govrix_scout_common::models::event::{AgentEvent, EventDirection, Provider};
    use uuid::Uuid;

    fn make_event() -> AgentEvent {
        AgentEvent::new(
            "agent-1",
            Uuid::now_v7(),
            EventDirection::Outbound,
            "POST",
            "https://api.openai.com/v1/chat/completions",
            Provider::OpenAI,
            "genesis",
            "audit:none",
        )
    }

    #[test]
    fn event_to_span_has_required_fields() {
        let event = make_event();
        let span = event_to_otlp_span(&event);
        assert!(span.get("traceId").is_some(), "traceId required");
        assert!(span.get("spanId").is_some(), "spanId required");
        assert!(span.get("name").is_some(), "name required");
        assert!(
            span.get("startTimeUnixNano").is_some(),
            "startTimeUnixNano required"
        );
        assert!(
            span.get("endTimeUnixNano").is_some(),
            "endTimeUnixNano required"
        );
        assert!(span.get("attributes").is_some(), "attributes required");
        assert!(span.get("status").is_some(), "status required");
    }

    #[test]
    fn span_id_is_16_hex_chars() {
        let event = make_event();
        let span = event_to_otlp_span(&event);
        let span_id = span["spanId"].as_str().unwrap();
        assert_eq!(
            span_id.len(),
            16,
            "OTel span_id must be 16 hex chars (8 bytes)"
        );
    }

    #[test]
    fn trace_id_is_32_hex_chars() {
        let event = make_event();
        let span = event_to_otlp_span(&event);
        let trace_id = span["traceId"].as_str().unwrap();
        assert_eq!(
            trace_id.len(),
            32,
            "OTel trace_id must be 32 hex chars (16 bytes)"
        );
    }

    #[test]
    fn events_to_otlp_request_structure() {
        let events = vec![make_event()];
        let req = events_to_otlp_request(&events, "test-service");

        assert!(req.get("resourceSpans").is_some());
        let resource_spans = req["resourceSpans"].as_array().unwrap();
        assert_eq!(resource_spans.len(), 1);

        let scope_spans = &resource_spans[0]["scopeSpans"];
        let spans = &scope_spans[0]["spans"];
        assert_eq!(spans.as_array().unwrap().len(), 1);
    }

    #[test]
    fn empty_events_produces_empty_spans_array() {
        let req = events_to_otlp_request(&[], "test-service");
        let spans = &req["resourceSpans"][0]["scopeSpans"][0]["spans"];
        assert!(spans.as_array().unwrap().is_empty());
    }

    #[test]
    fn status_ok_for_200() {
        let mut event = make_event();
        event.status_code = Some(200);
        let span = event_to_otlp_span(&event);
        assert_eq!(
            span["status"]["code"], 1,
            "HTTP 200 should map to OTel OK (1)"
        );
    }

    #[test]
    fn status_error_for_500() {
        let mut event = make_event();
        event.status_code = Some(500);
        let span = event_to_otlp_span(&event);
        assert_eq!(
            span["status"]["code"], 2,
            "HTTP 500 should map to OTel ERROR (2)"
        );
    }
}
