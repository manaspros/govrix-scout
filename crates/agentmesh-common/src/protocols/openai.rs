//! OpenAI protocol parser.
//!
//! Responsible for:
//! - Extracting model name, messages, tools, stream flag from request body
//! - Counting tokens from usage object in response
//! - Estimating cost using model pricing table
//! - Parsing SSE `data:` lines for streaming responses
//! - Extracting tool_calls, finish_reason, and content from responses

use serde::{Deserialize, Serialize};

/// OpenAI chat completion request (relevant fields only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiRequest {
    pub model: String,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub messages: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

/// OpenAI usage object from response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

/// A tool/function call in an OpenAI response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OpenAiFunction,
}

/// A function descriptor within a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiFunction {
    pub name: String,
    #[serde(default)]
    pub arguments: String,
}

/// A choice within an OpenAI chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiChoice {
    pub index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<OpenAiMessage>,
}

/// A message within an OpenAI response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiMessage {
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
}

/// OpenAI chat completion response (relevant fields only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiResponse {
    pub id: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAiUsage>,
    #[serde(default)]
    pub choices: Vec<OpenAiChoice>,
}

/// Parsed summary of an OpenAI response suitable for event logging.
#[derive(Debug, Clone)]
pub struct OpenAiParsedResponse {
    pub id: String,
    pub model: String,
    pub usage: Option<OpenAiUsage>,
    pub finish_reason: Option<String>,
    pub tool_calls: Vec<String>,
    pub content: Option<String>,
}

/// Parse the model name from an OpenAI request body.
///
/// Returns `None` if the body is not valid JSON or missing the `model` field.
pub fn parse_model(body: &[u8]) -> Option<String> {
    let req: OpenAiRequest = serde_json::from_slice(body).ok()?;
    Some(req.model)
}

/// Parse the full request for event logging.
///
/// Returns `None` if the body is not valid JSON.
pub fn parse_request(body: &[u8]) -> Option<OpenAiRequest> {
    serde_json::from_slice(body).ok()
}

/// Parse token usage from an OpenAI response body.
///
/// Returns `None` for streaming responses (usage is in the final chunk).
pub fn parse_usage(body: &[u8]) -> Option<OpenAiUsage> {
    let resp: OpenAiResponse = serde_json::from_slice(body).ok()?;
    resp.usage
}

/// Parse the full response into a structured summary.
pub fn parse_response(body: &[u8]) -> Option<OpenAiParsedResponse> {
    let resp: OpenAiResponse = serde_json::from_slice(body).ok()?;

    let finish_reason = resp.choices.first().and_then(|c| c.finish_reason.clone());

    let tool_calls = resp
        .choices
        .iter()
        .filter_map(|c| c.message.as_ref())
        .filter_map(|m| m.tool_calls.as_ref())
        .flatten()
        .map(|tc| tc.function.name.clone())
        .collect();

    let content = resp
        .choices
        .first()
        .and_then(|c| c.message.as_ref())
        .and_then(|m| m.content.clone());

    Some(OpenAiParsedResponse {
        id: resp.id,
        model: resp.model,
        usage: resp.usage,
        finish_reason,
        tool_calls,
        content,
    })
}

/// Check if a request body has `stream: true`.
pub fn is_streaming(body: &[u8]) -> bool {
    let Ok(req) = serde_json::from_slice::<OpenAiRequest>(body) else {
        return false;
    };
    req.stream
}

/// Extract tool calls from an OpenAI response value.
///
/// Returns the list of tool/function names called.
pub fn extract_tool_calls(response: &serde_json::Value) -> Vec<String> {
    let Some(choices) = response.get("choices").and_then(|c| c.as_array()) else {
        return Vec::new();
    };

    choices
        .iter()
        .filter_map(|choice| choice.get("message").or_else(|| choice.get("delta")))
        .filter_map(|msg| msg.get("tool_calls"))
        .filter_map(|tcs| tcs.as_array())
        .flatten()
        .filter_map(|tc| {
            tc.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from)
        })
        .collect()
}

/// Parse accumulated SSE chunks for a streaming OpenAI response.
///
/// Collects tool calls, finish_reason, and token usage from the chunks.
/// The final `data: [DONE]` chunk signals end of stream.
pub fn parse_streaming_chunks(chunks: &[String]) -> StreamingAccumulation {
    let mut accumulation = StreamingAccumulation::default();

    for chunk_str in chunks {
        let Ok(chunk) = serde_json::from_str::<serde_json::Value>(chunk_str) else {
            continue;
        };

        // Extract usage if present (some providers include it in the final delta)
        if let Some(usage) = chunk.get("usage") {
            if let (Some(prompt), Some(completion), Some(total)) = (
                usage.get("prompt_tokens").and_then(|v| v.as_i64()),
                usage.get("completion_tokens").and_then(|v| v.as_i64()),
                usage.get("total_tokens").and_then(|v| v.as_i64()),
            ) {
                accumulation.usage = Some(OpenAiUsage {
                    prompt_tokens: prompt as i32,
                    completion_tokens: completion as i32,
                    total_tokens: total as i32,
                });
            }
        }

        // Extract finish_reason and model from choices[0]
        if let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) {
            for choice in choices {
                if let Some(reason) = choice
                    .get("finish_reason")
                    .and_then(|r| r.as_str())
                    .filter(|r| !r.is_empty())
                {
                    accumulation.finish_reason = Some(reason.to_string());
                }

                // Accumulate tool call names from delta
                let msg = choice.get("delta").or_else(|| choice.get("message"));
                if let Some(tool_calls) = msg
                    .and_then(|m| m.get("tool_calls"))
                    .and_then(|t| t.as_array())
                {
                    for tc in tool_calls {
                        if let Some(name) = tc
                            .get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(|n| n.as_str())
                        {
                            if !accumulation.tool_calls.contains(&name.to_string()) {
                                accumulation.tool_calls.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Extract model if present
        if let Some(model) = chunk.get("model").and_then(|m| m.as_str()) {
            accumulation.model = Some(model.to_string());
        }
    }

    accumulation
}

/// Accumulation of data from streaming SSE chunks.
#[derive(Debug, Clone, Default)]
pub struct StreamingAccumulation {
    pub model: Option<String>,
    pub usage: Option<OpenAiUsage>,
    pub finish_reason: Option<String>,
    pub tool_calls: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_model_from_request() {
        let body = br#"{"model":"gpt-4o","messages":[{"role":"user","content":"hello"}]}"#;
        assert_eq!(parse_model(body), Some("gpt-4o".to_string()));
    }

    #[test]
    fn parse_model_returns_none_for_invalid_json() {
        assert_eq!(parse_model(b"not json"), None);
    }

    #[test]
    fn is_streaming_detects_true() {
        let body = br#"{"model":"gpt-4o","stream":true,"messages":[]}"#;
        assert!(is_streaming(body));
    }

    #[test]
    fn is_streaming_defaults_false() {
        let body = br#"{"model":"gpt-4o","messages":[]}"#;
        assert!(!is_streaming(body));
    }

    #[test]
    fn parse_usage_from_response() {
        let body = br#"{
            "id": "chatcmpl-abc",
            "model": "gpt-4o",
            "choices": [{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"Hi"}}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        }"#;
        let usage = parse_usage(body).unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn extract_tool_calls_from_response() {
        let body = serde_json::json!({
            "id": "chatcmpl-abc",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "finish_reason": "tool_calls",
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {"name": "get_weather", "arguments": "{}"}
                    }]
                }
            }]
        });
        let calls = extract_tool_calls(&body);
        assert_eq!(calls, vec!["get_weather"]);
    }

    #[test]
    fn parse_full_response() {
        let body = br#"{
            "id": "chatcmpl-xyz",
            "model": "gpt-4o-mini",
            "choices": [{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"Hello!"}}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8}
        }"#;
        let resp = parse_response(body).unwrap();
        assert_eq!(resp.model, "gpt-4o-mini");
        assert_eq!(resp.finish_reason, Some("stop".to_string()));
        assert_eq!(resp.content, Some("Hello!".to_string()));
        assert!(resp.usage.is_some());
    }
}
