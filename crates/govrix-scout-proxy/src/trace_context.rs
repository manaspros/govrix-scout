//! W3C Trace Context propagation (https://www.w3.org/TR/trace-context/).
//!
//! The proxy reads `traceparent` from inbound requests, stores the W3C trace ID
//! as `external_trace_id` for correlation with external tools, and injects a
//! new `traceparent` header on all forwarded requests so downstream services
//! (LLM providers, MCP servers) can link their spans back to Govrix.
//!
//! # Header format
//!
//! `traceparent: {version}-{trace-id}-{parent-id}-{flags}`
//! Example: `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01`
//!
//! - `version`: "00" (fixed)
//! - `trace-id`: 32 hex chars (128 bits)
//! - `parent-id`: 16 hex chars (64 bits)
//! - `flags`: "01" = sampled, "00" = not sampled

use uuid::Uuid;

/// Parsed W3C Trace Context from an inbound request.
#[derive(Debug, Clone)]
pub struct TraceContext {
    /// Govrix internal trace UUID (always freshly generated).
    pub trace_id: Uuid,

    /// W3C hex trace-id from the inbound `traceparent` header, if present.
    /// Stored in the `external_trace_id` column for external correlation.
    pub external_trace_id: Option<String>,

    /// W3C parent-id (hex) from the inbound `traceparent` header, if present.
    /// Becomes the `parent_span_id` of the first event in this request chain.
    pub w3c_parent_id: Option<String>,

    /// `tracestate` header value, if present (vendor-specific key=value pairs).
    pub tracestate: Option<String>,
}

impl TraceContext {
    /// Create a fresh context with no external correlation.
    pub fn new() -> Self {
        Self {
            trace_id: Uuid::now_v7(),
            external_trace_id: None,
            w3c_parent_id: None,
            tracestate: None,
        }
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse the `traceparent` header value and return a `TraceContext`.
///
/// If the header is present and valid, the W3C trace-id is stored as
/// `external_trace_id` (for correlation). A new internal `trace_id` is always
/// generated so Govrix traces are independent of the upstream trace hierarchy.
///
/// If the header is absent or malformed, a fresh context with no external
/// correlation is returned.
pub fn process_trace_context(
    traceparent: Option<&str>,
    tracestate: Option<&str>,
) -> TraceContext {
    let parsed = traceparent.and_then(parse_traceparent);

    match parsed {
        Some((ext_trace_id, parent_id)) => TraceContext {
            trace_id: Uuid::now_v7(),
            external_trace_id: Some(ext_trace_id),
            w3c_parent_id: Some(parent_id),
            tracestate: tracestate.map(str::to_string),
        },
        None => TraceContext {
            trace_id: Uuid::now_v7(),
            external_trace_id: None,
            w3c_parent_id: None,
            tracestate: tracestate.map(str::to_string),
        },
    }
}

/// Parse a W3C `traceparent` header value.
///
/// Format: `{version}-{trace-id}-{parent-id}-{flags}`
///
/// Returns `Some((trace_id_hex, parent_id_hex))` on success.
/// Returns `None` if the format is invalid.
pub fn parse_traceparent(value: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = value.splitn(4, '-').collect();
    if parts.len() != 4 {
        return None;
    }
    let version = parts[0];
    let trace_id = parts[1];
    let parent_id = parts[2];
    // parts[3] = flags — we don't need to inspect flags for basic propagation

    if version != "00" {
        return None;
    }
    // W3C trace-id: 32 hex chars; parent-id: 16 hex chars
    if trace_id.len() != 32 || parent_id.len() != 16 {
        return None;
    }
    // Basic hex validation — must be all hex digits
    if !trace_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    if !parent_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    Some((trace_id.to_string(), parent_id.to_string()))
}

/// Format a UUID as a W3C trace-id (32 lowercase hex chars, no hyphens).
pub fn format_w3c_trace_id(id: &Uuid) -> String {
    id.as_simple().to_string()
}

/// Format a UUID as a W3C parent-id / span-id (16 lowercase hex chars).
///
/// Uses the low 64 bits of the UUID (last 8 bytes).
pub fn format_w3c_span_id(id: &Uuid) -> String {
    let bytes = id.as_bytes();
    // Take the last 8 bytes as the 64-bit span ID
    hex::encode(&bytes[8..])
}

/// Build a `traceparent` header value for outbound requests.
///
/// This injects Govrix's internal trace_id and the current span_id so that
/// downstream services can link their spans back to Govrix.
///
/// Format: `00-{trace_id_32hex}-{span_id_16hex}-01`
pub fn build_traceparent(trace_id: &Uuid, span_id: &Uuid) -> String {
    format!(
        "00-{}-{}-01",
        format_w3c_trace_id(trace_id),
        format_w3c_span_id(span_id)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_traceparent() {
        let tp = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let result = parse_traceparent(tp).unwrap();
        assert_eq!(result.0, "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(result.1, "00f067aa0ba902b7");
    }

    #[test]
    fn parse_invalid_traceparent_wrong_version() {
        let tp = "01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        assert!(parse_traceparent(tp).is_none());
    }

    #[test]
    fn parse_invalid_traceparent_too_few_parts() {
        let tp = "00-4bf92f3577b34da6a3ce929d0e0e4736";
        assert!(parse_traceparent(tp).is_none());
    }

    #[test]
    fn parse_invalid_traceparent_wrong_trace_id_length() {
        let tp = "00-4bf92f35-00f067aa0ba902b7-01";
        assert!(parse_traceparent(tp).is_none());
    }

    #[test]
    fn process_trace_context_with_header() {
        let tp = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let ctx = process_trace_context(Some(tp), None);
        assert!(ctx.external_trace_id.is_some());
        assert_eq!(
            ctx.external_trace_id.unwrap(),
            "4bf92f3577b34da6a3ce929d0e0e4736"
        );
        assert!(ctx.w3c_parent_id.is_some());
    }

    #[test]
    fn process_trace_context_without_header() {
        let ctx = process_trace_context(None, None);
        assert!(ctx.external_trace_id.is_none());
        assert!(ctx.w3c_parent_id.is_none());
    }

    #[test]
    fn format_w3c_trace_id_is_32_chars() {
        let id = Uuid::now_v7();
        let formatted = format_w3c_trace_id(&id);
        assert_eq!(formatted.len(), 32);
        assert!(formatted.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn format_w3c_span_id_is_16_chars() {
        let id = Uuid::now_v7();
        let formatted = format_w3c_span_id(&id);
        assert_eq!(formatted.len(), 16);
        assert!(formatted.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn build_traceparent_correct_format() {
        let trace_id = Uuid::now_v7();
        let span_id = Uuid::now_v7();
        let tp = build_traceparent(&trace_id, &span_id);
        // Should be parseable
        let parsed = parse_traceparent(&tp);
        assert!(parsed.is_some());
    }
}
