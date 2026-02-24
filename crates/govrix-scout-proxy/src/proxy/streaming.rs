//! SSE streaming passthrough.
//!
//! Implements the SSE stream-through pattern from the rust-proxy skill:
//! - Streams chunks to the client immediately as they arrive from upstream
//! - Tees each chunk to the analysis pipeline via a bounded mpsc channel
//! - NEVER buffers the full SSE response before forwarding
//! - Uses `try_send` to drop analysis data rather than block the client
//!
//! Stream termination detection:
//! - OpenAI: `data: [DONE]`
//! - Anthropic: `event: message_stop`

use bytes::Bytes;
use tokio::sync::mpsc;

/// SSE chunk for the analysis pipeline.
///
/// Used by TeeSender for the SSE stream-through pattern.
/// Phase 1 will use this for real-time streaming with tokio channels.
#[allow(dead_code)]
#[derive(Debug)]
pub struct SseChunk {
    /// Raw bytes of the SSE chunk (may contain multiple `data:` lines).
    pub bytes: Bytes,
    /// Whether this is the final chunk (stream ended).
    pub is_final: bool,
}

/// Configuration for the analysis tee channel.
///
/// Phase 1 will use this for real-time SSE stream-through.
#[allow(dead_code)]
pub struct TeeSender {
    tx: mpsc::Sender<SseChunk>,
}

#[allow(dead_code)]
impl TeeSender {
    /// Create a new tee sender with a bounded channel.
    ///
    /// The channel capacity limits memory usage. When full, chunks are dropped
    /// (analysis is lossy) rather than blocking the streaming path.
    pub fn new(capacity: usize) -> (Self, mpsc::Receiver<SseChunk>) {
        let (tx, rx) = mpsc::channel(capacity);
        (Self { tx }, rx)
    }

    /// Send a chunk to the analysis pipeline without blocking.
    ///
    /// If the channel is full, the chunk is silently dropped.
    /// The proxy always forwards to the client regardless.
    pub fn tee(&self, bytes: Bytes, is_final: bool) {
        let _ = self.tx.try_send(SseChunk { bytes, is_final });
    }
}

/// Parse SSE `data:` lines from a raw chunk.
///
/// Returns a Vec of JSON strings extracted from `data:` lines.
/// Ignores comment lines (`:`) and event type lines (`event:`).
/// Filters out the `[DONE]` sentinel used by OpenAI.
pub fn parse_sse_data_lines(chunk: &[u8]) -> Vec<String> {
    let text = std::str::from_utf8(chunk).unwrap_or("");
    text.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .map(String::from)
        .collect()
}

/// Check if a chunk contains the OpenAI stream termination marker.
///
/// OpenAI signals end-of-stream with `data: [DONE]`.
#[allow(dead_code)]
pub fn is_openai_stream_done(chunk: &[u8]) -> bool {
    let text = std::str::from_utf8(chunk).unwrap_or("");
    text.lines().any(|line| line.trim() == "data: [DONE]")
}

/// Check if a chunk contains the Anthropic stream termination marker.
///
/// Anthropic signals end-of-stream with `event: message_stop`.
#[allow(dead_code)]
pub fn is_anthropic_stream_done(chunk: &[u8]) -> bool {
    let text = std::str::from_utf8(chunk).unwrap_or("");
    text.lines()
        .any(|line| line.trim() == "event: message_stop")
}

/// Check if a response content-type indicates SSE streaming.
#[allow(dead_code)]
pub fn is_sse_content_type(content_type: &str) -> bool {
    content_type.contains("text/event-stream")
}

/// Accumulated text from all SSE data lines in a streaming session.
///
/// Collects JSON payloads from `data:` lines as they arrive.
/// Used to reconstruct the full response for analysis after streaming ends.
pub struct SseAccumulator {
    /// All `data:` JSON strings collected so far (excluding [DONE]).
    #[allow(dead_code)]
    pub data_lines: Vec<String>,
    /// All raw Anthropic SSE events collected so far.
    pub anthropic_events: Vec<(
        govrix_scout_common::protocols::anthropic::AnthropicSseEvent,
        Option<serde_json::Value>,
    )>,
    /// Whether the stream has ended.
    pub is_done: bool,
}

impl SseAccumulator {
    pub fn new() -> Self {
        Self {
            data_lines: Vec::new(),
            anthropic_events: Vec::new(),
            is_done: false,
        }
    }

    /// Process a raw SSE chunk for OpenAI format.
    ///
    /// Extracts JSON data lines and detects [DONE].
    #[allow(dead_code)]
    pub fn process_openai_chunk(&mut self, chunk: &Bytes) {
        let lines = parse_sse_data_lines(chunk);
        self.data_lines.extend(lines);
        if is_openai_stream_done(chunk) {
            self.is_done = true;
        }
    }

    /// Process a raw SSE chunk for Anthropic format.
    ///
    /// Extracts event+data pairs and detects message_stop.
    pub fn process_anthropic_chunk(&mut self, chunk: &Bytes) {
        let events = govrix_scout_common::protocols::anthropic::parse_sse_events(chunk);
        for (event, data) in events {
            if event == govrix_scout_common::protocols::anthropic::AnthropicSseEvent::MessageStop {
                self.is_done = true;
            }
            self.anthropic_events.push((event, data));
        }
    }
}

impl Default for SseAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sse_data_lines_extracts_json() {
        let chunk = b"data: {\"id\":\"1\",\"delta\":\"hello\"}\ndata: {\"id\":\"2\",\"delta\":\" world\"}\ndata: [DONE]\n";
        let lines = parse_sse_data_lines(chunk);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("hello"));
        assert!(lines[1].contains("world"));
    }

    #[test]
    fn tee_sender_does_not_block_on_full_channel() {
        let (tee, _rx) = TeeSender::new(2);
        // Send 5 chunks into a capacity-2 channel — must not panic or block
        for i in 0..5 {
            tee.tee(Bytes::from(format!("chunk {}", i)), i == 4);
        }
    }

    #[test]
    fn openai_done_detection() {
        let chunk_done = b"data: {\"id\":\"1\"}\n\ndata: [DONE]\n\n";
        assert!(is_openai_stream_done(chunk_done));

        let chunk_not_done = b"data: {\"id\":\"1\"}\n\n";
        assert!(!is_openai_stream_done(chunk_not_done));
    }

    #[test]
    fn anthropic_done_detection() {
        let chunk_done = b"event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
        assert!(is_anthropic_stream_done(chunk_done));

        let chunk_not_done =
            b"event: content_block_delta\ndata: {\"type\":\"content_block_delta\"}\n\n";
        assert!(!is_anthropic_stream_done(chunk_not_done));
    }

    #[test]
    fn sse_accumulator_openai() {
        let mut acc = SseAccumulator::new();

        acc.process_openai_chunk(&Bytes::from(
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\n",
        ));
        assert!(!acc.is_done);
        assert_eq!(acc.data_lines.len(), 1);

        acc.process_openai_chunk(&Bytes::from("data: [DONE]\n\n"));
        assert!(acc.is_done);
    }

    #[test]
    fn sse_accumulator_anthropic() {
        let mut acc = SseAccumulator::new();

        acc.process_anthropic_chunk(&Bytes::from(
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\"}\n\n",
        ));
        assert!(!acc.is_done);

        acc.process_anthropic_chunk(&Bytes::from(
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
        ));
        assert!(acc.is_done);
    }

    #[test]
    fn is_sse_content_type_detection() {
        assert!(is_sse_content_type("text/event-stream; charset=utf-8"));
        assert!(is_sse_content_type("text/event-stream"));
        assert!(!is_sse_content_type("application/json"));
    }
}
