//! Anthropic Messages API protocol parser.
//!
//! Responsible for:
//! - Extracting model name, messages, max_tokens, stream flag from request body
//! - Counting tokens from usage object in response
//! - Estimating cost using model pricing table
//! - Parsing SSE `event:` + `data:` lines for streaming responses
//! - Extracting tool_use blocks, stop_reason, and content blocks from responses

use serde::{Deserialize, Serialize};

/// Anthropic messages request (relevant fields only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub max_tokens: i32,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub messages: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

/// Anthropic usage object (present in both streaming and non-streaming responses).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

/// A content block in an Anthropic response.
///
/// Anthropic uses typed content blocks: "text", "tool_use", "image", etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    /// Present for "text" blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Present for "tool_use" blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Present for "tool_use" blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Present for "tool_use" blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
}

/// Anthropic messages response (relevant fields only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    pub model: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub stop_reason: Option<String>,
    pub usage: AnthropicUsage,
    #[serde(default)]
    pub content: Vec<AnthropicContentBlock>,
}

/// Parsed summary of an Anthropic response suitable for event logging.
#[derive(Debug, Clone)]
pub struct AnthropicParsedResponse {
    pub id: String,
    pub model: String,
    pub usage: AnthropicUsage,
    pub stop_reason: Option<String>,
    pub tool_calls: Vec<String>,
    pub text_content: Option<String>,
}

/// Parse the model name from an Anthropic request body.
pub fn parse_model(body: &[u8]) -> Option<String> {
    let req: AnthropicRequest = serde_json::from_slice(body).ok()?;
    Some(req.model)
}

/// Parse the full request for event logging.
pub fn parse_request(body: &[u8]) -> Option<AnthropicRequest> {
    serde_json::from_slice(body).ok()
}

/// Parse token usage from an Anthropic response body.
pub fn parse_usage(body: &[u8]) -> Option<AnthropicUsage> {
    let resp: AnthropicResponse = serde_json::from_slice(body).ok()?;
    Some(resp.usage)
}

/// Parse the full response into a structured summary.
pub fn parse_response(body: &[u8]) -> Option<AnthropicParsedResponse> {
    let resp: AnthropicResponse = serde_json::from_slice(body).ok()?;

    let tool_calls: Vec<String> = resp
        .content
        .iter()
        .filter(|block| block.block_type == "tool_use")
        .filter_map(|block| block.name.clone())
        .collect();

    let text_content = resp
        .content
        .iter()
        .find(|block| block.block_type == "text")
        .and_then(|block| block.text.clone());

    Some(AnthropicParsedResponse {
        id: resp.id,
        model: resp.model,
        usage: resp.usage,
        stop_reason: resp.stop_reason,
        tool_calls,
        text_content,
    })
}

/// Check if a request body has `stream: true`.
pub fn is_streaming(body: &[u8]) -> bool {
    let Ok(req) = serde_json::from_slice::<AnthropicRequest>(body) else {
        return false;
    };
    req.stream
}

/// Extract tool use blocks from an Anthropic response value.
///
/// Returns the list of tool names called.
pub fn extract_tool_calls(response: &serde_json::Value) -> Vec<String> {
    let Some(content) = response.get("content").and_then(|c| c.as_array()) else {
        return Vec::new();
    };

    content
        .iter()
        .filter(|block| {
            block
                .get("type")
                .and_then(|t| t.as_str())
                .map(|t| t == "tool_use")
                .unwrap_or(false)
        })
        .filter_map(|block| block.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect()
}

/// Streaming SSE event types for Anthropic.
///
/// Anthropic's streaming format uses `event:` lines followed by `data:` JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnthropicSseEvent {
    /// `event: message_start` — contains initial message metadata and usage.
    MessageStart,
    /// `event: content_block_start` — starts a new content block.
    ContentBlockStart,
    /// `event: content_block_delta` — incremental content.
    ContentBlockDelta,
    /// `event: content_block_stop` — ends a content block.
    ContentBlockStop,
    /// `event: message_delta` — contains stop_reason and output token usage.
    MessageDelta,
    /// `event: message_stop` — final event, stream is complete.
    MessageStop,
    /// `event: ping` — heartbeat.
    Ping,
    /// Unknown event type.
    Unknown(String),
}

impl From<&str> for AnthropicSseEvent {
    fn from(s: &str) -> Self {
        match s {
            "message_start" => AnthropicSseEvent::MessageStart,
            "content_block_start" => AnthropicSseEvent::ContentBlockStart,
            "content_block_delta" => AnthropicSseEvent::ContentBlockDelta,
            "content_block_stop" => AnthropicSseEvent::ContentBlockStop,
            "message_delta" => AnthropicSseEvent::MessageDelta,
            "message_stop" => AnthropicSseEvent::MessageStop,
            "ping" => AnthropicSseEvent::Ping,
            other => AnthropicSseEvent::Unknown(other.to_string()),
        }
    }
}

/// Parse Anthropic SSE chunks (event + data pairs).
///
/// Returns a vec of `(event_type, data_json)` pairs.
pub fn parse_sse_events(chunk: &[u8]) -> Vec<(AnthropicSseEvent, Option<serde_json::Value>)> {
    let text = std::str::from_utf8(chunk).unwrap_or("");
    let mut results = Vec::new();
    let mut current_event: Option<AnthropicSseEvent> = None;

    for line in text.lines() {
        if let Some(event_str) = line.strip_prefix("event: ") {
            current_event = Some(AnthropicSseEvent::from(event_str.trim()));
        } else if let Some(data_str) = line.strip_prefix("data: ") {
            let data = serde_json::from_str(data_str).ok();
            if let Some(event) = current_event.take() {
                results.push((event, data));
            }
        }
    }

    results
}

/// Accumulate data from Anthropic streaming SSE chunks.
///
/// Tracks usage (from message_start and message_delta), stop_reason,
/// and tool_use blocks encountered across all chunks.
#[derive(Debug, Clone, Default)]
pub struct AnthropicStreamingAccumulation {
    pub model: Option<String>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub stop_reason: Option<String>,
    pub tool_calls: Vec<String>,
}

impl AnthropicStreamingAccumulation {
    /// Get unified usage if both token counts are available.
    pub fn usage(&self) -> Option<AnthropicUsage> {
        match (self.input_tokens, self.output_tokens) {
            (Some(input), Some(output)) => Some(AnthropicUsage {
                input_tokens: input,
                output_tokens: output,
            }),
            _ => None,
        }
    }
}

/// Process accumulated SSE event data into a streaming accumulation.
pub fn accumulate_streaming_events(
    events: &[(AnthropicSseEvent, Option<serde_json::Value>)],
) -> AnthropicStreamingAccumulation {
    let mut acc = AnthropicStreamingAccumulation::default();

    for (event, data) in events {
        let Some(data) = data else { continue };

        match event {
            AnthropicSseEvent::MessageStart => {
                // message_start data: {"type":"message_start","message":{...,"usage":{...}}}
                if let Some(msg) = data.get("message") {
                    if let Some(model) = msg.get("model").and_then(|m| m.as_str()) {
                        acc.model = Some(model.to_string());
                    }
                    if let Some(usage) = msg.get("usage") {
                        if let Some(input) = usage.get("input_tokens").and_then(|v| v.as_i64()) {
                            acc.input_tokens = Some(input as i32);
                        }
                    }
                }
            }
            AnthropicSseEvent::ContentBlockStart => {
                // content_block_start: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","name":"...","id":"..."}}
                if let Some(block) = data.get("content_block") {
                    if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                        if let Some(name) = block.get("name").and_then(|n| n.as_str()) {
                            if !acc.tool_calls.contains(&name.to_string()) {
                                acc.tool_calls.push(name.to_string());
                            }
                        }
                    }
                }
            }
            AnthropicSseEvent::MessageDelta => {
                // message_delta data: {"type":"message_delta","delta":{"stop_reason":"...","stop_sequence":null},"usage":{"output_tokens":N}}
                if let Some(delta) = data.get("delta") {
                    if let Some(reason) = delta.get("stop_reason").and_then(|r| r.as_str()) {
                        acc.stop_reason = Some(reason.to_string());
                    }
                }
                if let Some(usage) = data.get("usage") {
                    if let Some(output) = usage.get("output_tokens").and_then(|v| v.as_i64()) {
                        acc.output_tokens = Some(output as i32);
                    }
                }
            }
            _ => {}
        }
    }

    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_model_from_request() {
        let body = br#"{"model":"claude-3-5-sonnet-20241022","max_tokens":1024,"messages":[{"role":"user","content":"hello"}]}"#;
        assert_eq!(
            parse_model(body),
            Some("claude-3-5-sonnet-20241022".to_string())
        );
    }

    #[test]
    fn is_streaming_detects_true() {
        let body = br#"{"model":"claude-3-5-sonnet-20241022","max_tokens":1024,"stream":true,"messages":[]}"#;
        assert!(is_streaming(body));
    }

    #[test]
    fn is_streaming_defaults_false() {
        let body = br#"{"model":"claude-3-5-sonnet-20241022","max_tokens":1024,"messages":[]}"#;
        assert!(!is_streaming(body));
    }

    #[test]
    fn parse_usage_from_response() {
        let body = br#"{
            "id": "msg_abc",
            "type": "message",
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 20, "output_tokens": 10},
            "content": [{"type":"text","text":"Hi there"}]
        }"#;
        let usage = parse_usage(body).unwrap();
        assert_eq!(usage.input_tokens, 20);
        assert_eq!(usage.output_tokens, 10);
    }

    #[test]
    fn extract_tool_calls_from_response() {
        let body = serde_json::json!({
            "id": "msg_abc",
            "type": "message",
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 30, "output_tokens": 5},
            "content": [
                {"type": "tool_use", "id": "toolu_1", "name": "get_weather", "input": {}}
            ]
        });
        let calls = extract_tool_calls(&body);
        assert_eq!(calls, vec!["get_weather"]);
    }

    #[test]
    fn parse_sse_message_stop() {
        let chunk = b"event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
        let events = parse_sse_events(chunk);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, AnthropicSseEvent::MessageStop);
    }

    #[test]
    fn parse_sse_message_delta_with_stop_reason() {
        let chunk = b"event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":42}}\n\n";
        let events = parse_sse_events(chunk);
        assert_eq!(events.len(), 1);

        let acc = accumulate_streaming_events(&events);
        assert_eq!(acc.stop_reason, Some("end_turn".to_string()));
        assert_eq!(acc.output_tokens, Some(42));
    }

    #[test]
    fn parse_full_response() {
        let body = br#"{
            "id": "msg_xyz",
            "type": "message",
            "model": "claude-3-haiku-20240307",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 5, "output_tokens": 8},
            "content": [{"type":"text","text":"Hello world"}]
        }"#;
        let resp = parse_response(body).unwrap();
        assert_eq!(resp.model, "claude-3-haiku-20240307");
        assert_eq!(resp.stop_reason, Some("end_turn".to_string()));
        assert_eq!(resp.text_content, Some("Hello world".to_string()));
        assert!(resp.tool_calls.is_empty());
    }
}
