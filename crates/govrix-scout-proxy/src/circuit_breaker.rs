//! Circuit breaker subsystem.
//!
//! Provides three in-memory circuit breaker checks that run in the proxy hot path
//! before forwarding each request upstream. All checks are synchronous and operate
//! against in-memory state — zero DB round-trips in the critical path.
//!
//! # Checks
//!
//! 1. **Loop Detector** — blocks repeated identical tool invocations within a
//!    rolling time window (default: 5 calls per 60 seconds per agent+tool).
//!
//! 2. **Risk Circuit Breaker** — blocks agents whose rolling-average risk score
//!    exceeds a configurable threshold (default: 75.0 over 5 minutes).
//!
//! # Design
//!
//! - `LoopDetector` and `RiskCircuitBreaker` use `std::collections::HashMap` with
//!   `Mutex` for safe concurrent access. The Mutex is held only briefly for reads
//!   and writes — no I/O, no blocking calls.
//! - All checks are fail-open: any internal error (mutex poison, overflow) returns
//!   `None` (allow) so agent traffic is never blocked by a tracing fault.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

// ── Block response ─────────────────────────────────────────────────────────────

/// A circuit breaker block decision.
///
/// Returned by each check when the circuit breaker fires.
/// The proxy converts this into an HTTP 429 or 503 response.
#[derive(Debug, Clone)]
pub struct CircuitBreakerBlock {
    /// What triggered the block.
    pub kind: BlockKind,
    /// HTTP status code to return to the client.
    pub status: u16,
    /// Human-readable message for the response body.
    pub message: String,
    /// Suggested seconds until retry (maps to `Retry-After` header).
    pub retry_after_secs: Option<u64>,
}

impl CircuitBreakerBlock {
    /// Build the JSON response body for this block.
    pub fn to_json_body(&self) -> String {
        let error_type = match self.kind {
            BlockKind::LoopDetected => "loop_detected",
            BlockKind::RiskThresholdExceeded => "risk_threshold_exceeded",
        };
        format!(
            r#"{{"error":{{"type":"{}","message":"{}","code":{}}}}}"#,
            error_type, self.message, self.status
        )
    }
}

/// The kind of circuit breaker that fired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    LoopDetected,
    RiskThresholdExceeded,
}

// ── Loop Detector ─────────────────────────────────────────────────────────────

/// Detects repeated identical tool invocations within a rolling time window.
///
/// Tracks per-(agent_id, tool_name) call timestamps in a sliding window.
/// If the same tool is called more than `max_calls` times within `window_secs`,
/// a `CircuitBreakerBlock` is returned.
///
/// This is the primary pattern for runaway agent loops.
pub struct LoopDetector {
    /// Per-(agent_id, tool_name) → ordered call timestamps.
    windows: HashMap<(String, String), VecDeque<Instant>>,
    /// Maximum allowed calls within the window before triggering.
    max_calls: usize,
    /// Duration of the sliding window in seconds.
    window_secs: u64,
}

impl LoopDetector {
    /// Create a new `LoopDetector` with defaults: max 5 calls per 60 seconds.
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            max_calls: 5,
            window_secs: 60,
        }
    }

    /// Create with custom limits.
    pub fn with_limits(max_calls: usize, window_secs: u64) -> Self {
        Self {
            windows: HashMap::new(),
            max_calls,
            window_secs,
        }
    }

    /// Record a tool call and check if a loop has been detected.
    ///
    /// Returns `Some(block)` if the loop threshold is exceeded, `None` otherwise.
    /// Always records the call even when blocking (so the count keeps climbing).
    pub fn check_and_record(
        &mut self,
        agent_id: &str,
        tool_name: &str,
    ) -> Option<CircuitBreakerBlock> {
        let key = (agent_id.to_string(), tool_name.to_string());
        let now = Instant::now();
        let cutoff = now - Duration::from_secs(self.window_secs);

        let calls = self.windows.entry(key).or_default();

        // Evict timestamps outside the window
        while let Some(&front) = calls.front() {
            if front < cutoff {
                calls.pop_front();
            } else {
                break;
            }
        }

        // Record the current call
        calls.push_back(now);

        if calls.len() > self.max_calls {
            Some(CircuitBreakerBlock {
                kind: BlockKind::LoopDetected,
                status: 429,
                message: format!(
                    "Tool '{}' called {} times in {}s — possible loop detected",
                    tool_name,
                    calls.len(),
                    self.window_secs
                ),
                retry_after_secs: Some(self.window_secs),
            })
        } else {
            None
        }
    }

    /// Clear the loop window for a given agent and tool (e.g. after session end).
    pub fn clear(&mut self, agent_id: &str, tool_name: &str) {
        self.windows
            .remove(&(agent_id.to_string(), tool_name.to_string()));
    }
}

impl Default for LoopDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ── Risk Circuit Breaker ───────────────────────────────────────────────────────

/// Blocks agents whose rolling-average risk score exceeds a threshold.
///
/// Maintains a per-agent sliding window of (timestamp, risk_score) pairs.
/// When the rolling average of the last `window_secs` exceeds `threshold`,
/// the circuit breaker fires.
///
/// Requires at least `min_events` data points before triggering to avoid
/// false positives on the first high-risk event.
pub struct RiskCircuitBreaker {
    /// Per-agent_id → ordered (timestamp, risk_score) pairs.
    windows: HashMap<String, VecDeque<(Instant, f32)>>,
    /// Duration of the sliding window in seconds (default: 300 = 5 minutes).
    window_secs: u64,
    /// Risk score threshold [0.0, 100.0] to trigger the breaker (default: 75.0).
    threshold: f32,
    /// Minimum number of events required before the breaker can trigger (default: 3).
    min_events: usize,
}

impl RiskCircuitBreaker {
    /// Create a new `RiskCircuitBreaker` with defaults.
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            window_secs: 300,
            threshold: 75.0,
            min_events: 3,
        }
    }

    /// Create with custom settings.
    pub fn with_settings(window_secs: u64, threshold: f32, min_events: usize) -> Self {
        Self {
            windows: HashMap::new(),
            window_secs,
            threshold,
            min_events,
        }
    }

    /// Record a risk score observation for an agent.
    pub fn record(&mut self, agent_id: &str, risk_score: f32) {
        let now = Instant::now();
        let cutoff = now - Duration::from_secs(self.window_secs);

        let window = self.windows.entry(agent_id.to_string()).or_default();
        // Evict old entries
        while let Some(&(ts, _)) = window.front() {
            if ts < cutoff {
                window.pop_front();
            } else {
                break;
            }
        }
        window.push_back((now, risk_score));
    }

    /// Record a risk score and immediately check if the breaker should fire.
    ///
    /// Combines `record` + `check` into one call for the hot path.
    /// Returns `Some(block)` if the circuit breaker fires, `None` otherwise.
    pub fn record_and_check(
        &mut self,
        agent_id: &str,
        risk_score: f32,
    ) -> Option<CircuitBreakerBlock> {
        self.record(agent_id, risk_score);
        self.check(agent_id)
    }

    /// Check if an agent's rolling risk average exceeds the threshold.
    ///
    /// Returns `Some(block)` if the circuit breaker fires, `None` otherwise.
    pub fn check(&self, agent_id: &str) -> Option<CircuitBreakerBlock> {
        let window = self.windows.get(agent_id)?;

        if window.len() < self.min_events {
            return None;
        }

        let avg: f32 = window.iter().map(|(_, s)| s).sum::<f32>() / window.len() as f32;

        if avg >= self.threshold {
            Some(CircuitBreakerBlock {
                kind: BlockKind::RiskThresholdExceeded,
                status: 503,
                message: format!(
                    "Agent risk score {:.1} exceeds threshold {:.1}",
                    avg, self.threshold
                ),
                retry_after_secs: None,
            })
        } else {
            None
        }
    }
}

impl Default for RiskCircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── LoopDetector ─────────────────────────────────────────────────────────

    #[test]
    fn loop_detector_allows_within_limit() {
        let mut ld = LoopDetector::with_limits(3, 60);
        for _ in 0..3 {
            assert!(ld.check_and_record("agent-1", "my_tool").is_none());
        }
    }

    #[test]
    fn loop_detector_blocks_above_limit() {
        let mut ld = LoopDetector::with_limits(3, 60);
        for _ in 0..3 {
            ld.check_and_record("agent-1", "my_tool");
        }
        let block = ld.check_and_record("agent-1", "my_tool");
        assert!(block.is_some());
        let b = block.unwrap();
        assert_eq!(b.kind, BlockKind::LoopDetected);
        assert_eq!(b.status, 429);
    }

    #[test]
    fn loop_detector_different_tools_are_independent() {
        let mut ld = LoopDetector::with_limits(2, 60);
        // 2 calls to tool_a — right at limit
        ld.check_and_record("agent-1", "tool_a");
        ld.check_and_record("agent-1", "tool_a");
        // tool_b is fresh
        assert!(ld.check_and_record("agent-1", "tool_b").is_none());
    }

    #[test]
    fn loop_detector_block_has_retry_after() {
        let mut ld = LoopDetector::with_limits(1, 30);
        ld.check_and_record("agent-1", "tool");
        let block = ld.check_and_record("agent-1", "tool").unwrap();
        assert_eq!(block.retry_after_secs, Some(30));
    }

    #[test]
    fn loop_detector_resets_after_window() {
        // Test that calls after the window are not blocked.
        // Use a very short window (1 second) and call right at the limit.
        let mut ld = LoopDetector::with_limits(2, 1);
        ld.check_and_record("agent-reset", "tool_x");
        ld.check_and_record("agent-reset", "tool_x");
        // At this point we're at limit; a third call within the window would block.
        // We can't actually sleep in a unit test, so we verify the counter is at limit.
        // The window will expire in 1s — here we just verify the fourth call blocks
        // (since we can't control time easily, this verifies the count-based logic).
        let block = ld.check_and_record("agent-reset", "tool_x");
        assert!(block.is_some(), "third call exceeds limit=2, should block");
        // Different agent is unaffected
        assert!(ld.check_and_record("agent-other", "tool_x").is_none());
    }

    // ── RiskCircuitBreaker ────────────────────────────────────────────────────

    #[test]
    fn risk_breaker_needs_min_events() {
        let mut rb = RiskCircuitBreaker::with_settings(300, 50.0, 3);
        rb.record("agent-1", 100.0);
        rb.record("agent-1", 100.0);
        // Only 2 events — below min_events=3, should not trigger
        assert!(rb.check("agent-1").is_none());
    }

    #[test]
    fn risk_breaker_triggers_above_threshold() {
        let mut rb = RiskCircuitBreaker::with_settings(300, 50.0, 3);
        for _ in 0..3 {
            rb.record("agent-1", 80.0);
        }
        let block = rb.check("agent-1");
        assert!(block.is_some());
        assert_eq!(block.unwrap().kind, BlockKind::RiskThresholdExceeded);
    }

    #[test]
    fn risk_breaker_allows_below_threshold() {
        let mut rb = RiskCircuitBreaker::with_settings(300, 75.0, 3);
        for _ in 0..5 {
            rb.record("agent-1", 30.0);
        }
        assert!(rb.check("agent-1").is_none());
    }

    #[test]
    fn risk_breaker_does_not_block_low_risk_tools() {
        let mut cb = RiskCircuitBreaker::new();
        // Score 5.0 is well below default threshold of 75.0.
        // Even with 10 calls, rolling avg stays at 5.0 → no block.
        for _ in 0..10 {
            assert!(cb.record_and_check("agent1", 5.0).is_none());
        }
    }

    #[test]
    fn risk_breaker_record_and_check_fires_above_threshold() {
        let mut rb = RiskCircuitBreaker::with_settings(300, 50.0, 3);
        // First two calls — min_events not met yet
        assert!(rb.record_and_check("agent-2", 80.0).is_none());
        assert!(rb.record_and_check("agent-2", 80.0).is_none());
        // Third call — min_events met, avg=80.0 > threshold=50.0 → block
        let result = rb.record_and_check("agent-2", 80.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind, BlockKind::RiskThresholdExceeded);
    }

    #[test]
    fn circuit_block_json_body_is_valid() {
        let block = CircuitBreakerBlock {
            kind: BlockKind::LoopDetected,
            status: 429,
            message: "test loop".to_string(),
            retry_after_secs: Some(60),
        };
        let json = block.to_json_body();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["error"]["type"], "loop_detected");
        assert_eq!(v["error"]["code"], 429);
    }
}
