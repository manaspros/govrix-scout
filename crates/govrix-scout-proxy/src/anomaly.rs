//! Anomaly detection for AI agent behaviour.
//!
//! Detectors run against the event stream (NOT the hot path) to flag unusual
//! patterns without ever blocking traffic.
//!
//! # Design invariants
//!
//! - **Non-blocking**: detectors are called from `flush_batch` after DB writes.
//! - **Fail-open**: a panicking detector must never crash the background writer.
//! - **No state persistence**: detector state is in-memory only; it resets on
//!   proxy restart. Future work: seed from DB on startup.
//!
//! # Current detectors
//!
//! | Detector | Triggers on |
//! |----------|------------|
//! | `OffHoursDetector` | Agent active outside 06:00–22:00 UTC |
//! | `TokenVolumeDetector` | Tokens this event > 5× EMA |
//! | `NewToolDetector` | Agent calling a tool it has never called before (after warm-up) |

use std::collections::HashMap;

use chrono::{DateTime, Timelike, Utc};
use govrix_scout_common::models::event::{AgentEvent, EventKind};

// ── Alert types ───────────────────────────────────────────────────────────────

/// A single anomaly alert produced by one of the detectors.
#[derive(Debug, Clone)]
pub struct AnomalyAlert {
    /// The agent this alert concerns.
    pub agent_id: String,
    /// What kind of anomaly was detected.
    pub anomaly_type: AnomalyType,
    /// Severity level.
    pub severity: AnomalySeverity,
    /// Human-readable description including metrics.
    pub description: String,
    /// When the anomaly was detected (wallclock, UTC).
    pub detected_at: DateTime<Utc>,
    /// Structured evidence for downstream consumers (DB, SIEM, UI).
    pub evidence: serde_json::Value,
}

/// Anomaly classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnomalyType {
    /// Agent active outside normal working hours.
    OffHoursUsage,
    /// Token usage for a single event is more than `spike_threshold`× EMA.
    UnusualTokenVolume,
    /// Agent called a tool it has never called before (after warm-up).
    NewToolFirstSeen,
    /// Agent switched to a different LLM provider than its historical norm.
    UnusualProvider,
    /// Cost per event exceeds a multiple of the rolling average.
    RapidCostEscalation,
}

/// Alert severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnomalySeverity {
    /// Informational — unusual but not alarming.
    Info,
    /// Worth investigating.
    Warning,
    /// Immediate attention required.
    Critical,
}

// ── OffHoursDetector ─────────────────────────────────────────────────────────

/// Detects agent activity outside business hours (UTC).
///
/// Default window: 06:00–22:00 UTC (configurable).
pub struct OffHoursDetector {
    /// Inclusive start of business hours (0–23, UTC).
    pub business_hours_start: u32,
    /// Exclusive end of business hours (0–23, UTC).
    pub business_hours_end: u32,
}

impl OffHoursDetector {
    /// Create a detector with the default 06:00–22:00 UTC window.
    pub fn new() -> Self {
        Self {
            business_hours_start: 6,
            business_hours_end: 22,
        }
    }

    /// Check an event. Returns `Some(alert)` if the event falls outside hours.
    pub fn check(&self, event: &AgentEvent) -> Option<AnomalyAlert> {
        let hour = event.timestamp.hour();
        if hour < self.business_hours_start || hour >= self.business_hours_end {
            Some(AnomalyAlert {
                agent_id: event.agent_id.clone(),
                anomaly_type: AnomalyType::OffHoursUsage,
                severity: AnomalySeverity::Info,
                description: format!(
                    "Agent '{}' active at {:02}:00 UTC (outside {}:00-{}:00 business hours)",
                    event.agent_id, hour, self.business_hours_start, self.business_hours_end
                ),
                detected_at: Utc::now(),
                evidence: serde_json::json!({
                    "hour_utc": hour,
                    "event_kind": event.event_kind.to_string(),
                }),
            })
        } else {
            None
        }
    }
}

impl Default for OffHoursDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ── TokenVolumeDetector ───────────────────────────────────────────────────────

/// Detects unusually high token usage using an exponential moving average (EMA).
///
/// Once the warm-up period has passed, fires if `tokens > spike_threshold × EMA`.
pub struct TokenVolumeDetector {
    /// agent_id → EMA of (input_tokens + output_tokens) per event.
    ema: HashMap<String, f64>,
    /// agent_id → number of events processed (for warm-up tracking).
    count: HashMap<String, u32>,
    /// EMA smoothing factor α (0 < α < 1). Smaller = slower adaptation.
    pub alpha: f64,
    /// Spike threshold multiplier. Fire if `tokens > spike_threshold × EMA`.
    pub spike_threshold: f64,
    /// Number of events before triggering (warm-up period).
    pub warmup_events: u32,
}

impl TokenVolumeDetector {
    /// Create a detector with sensible defaults:
    /// α = 0.1, spike = 5×, warm-up = 10 events.
    pub fn new() -> Self {
        Self {
            ema: HashMap::new(),
            count: HashMap::new(),
            alpha: 0.1,
            spike_threshold: 5.0,
            warmup_events: 10,
        }
    }

    /// Check an event and update the EMA. Returns `Some(alert)` on spike.
    pub fn check(&mut self, event: &AgentEvent) -> Option<AnomalyAlert> {
        let tokens = (event.input_tokens.unwrap_or(0) + event.output_tokens.unwrap_or(0)) as f64;
        if tokens == 0.0 {
            return None;
        }

        let count = self.count.entry(event.agent_id.clone()).or_insert(0);
        let ema = self.ema.entry(event.agent_id.clone()).or_insert(tokens);
        *count += 1;

        let alert = if *count > self.warmup_events && tokens > self.spike_threshold * *ema {
            Some(AnomalyAlert {
                agent_id: event.agent_id.clone(),
                anomaly_type: AnomalyType::UnusualTokenVolume,
                severity: AnomalySeverity::Warning,
                description: format!(
                    "Agent '{}' used {:.0} tokens (EMA: {:.0}, spike: {:.1}×)",
                    event.agent_id,
                    tokens,
                    *ema,
                    tokens / *ema
                ),
                detected_at: Utc::now(),
                evidence: serde_json::json!({
                    "tokens_this_event": tokens,
                    "ema_tokens": *ema,
                    "spike_ratio": tokens / *ema,
                    "event_kind": event.event_kind.to_string(),
                }),
            })
        } else {
            None
        };

        // Update EMA after the check (so the spike itself doesn't distort the EMA immediately)
        *ema = *ema * (1.0 - self.alpha) + tokens * self.alpha;

        alert
    }
}

impl Default for TokenVolumeDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ── NewToolDetector ───────────────────────────────────────────────────────────

/// Detects when an agent calls a tool for the first time.
///
/// The first `warmup_tools` distinct tools are silently learned. After that,
/// each new tool name produces an `Info` alert.
pub struct NewToolDetector {
    /// agent_id → set of known tool names.
    known_tools: HashMap<String, std::collections::HashSet<String>>,
    /// Number of tools to learn silently before alerting.
    pub warmup_tools: usize,
}

impl NewToolDetector {
    /// Create a detector with a warm-up window of 3 tools.
    pub fn new() -> Self {
        Self {
            known_tools: HashMap::new(),
            warmup_tools: 3,
        }
    }

    /// Check a `tool.invoke` event. Updates internal state and returns
    /// `Some(alert)` when a new tool is seen after the warm-up window.
    pub fn check(&mut self, event: &AgentEvent) -> Option<AnomalyAlert> {
        let tool_name = event.tool_name.as_ref()?;

        // Only check tool.invoke events
        if event.event_kind != EventKind::ToolInvoke {
            return None;
        }

        let known = self.known_tools.entry(event.agent_id.clone()).or_default();

        if known.contains(tool_name) {
            return None; // already seen — no alert
        }

        // Register the new tool
        known.insert(tool_name.clone());

        // Silently learn the first `warmup_tools` tools
        if known.len() <= self.warmup_tools {
            return None;
        }

        Some(AnomalyAlert {
            agent_id: event.agent_id.clone(),
            anomaly_type: AnomalyType::NewToolFirstSeen,
            severity: AnomalySeverity::Info,
            description: format!(
                "Agent '{}' called new tool '{}' for the first time",
                event.agent_id, tool_name
            ),
            detected_at: Utc::now(),
            evidence: serde_json::json!({
                "tool_name": tool_name,
                "mcp_server": event.mcp_server,
                "total_distinct_tools": known.len(),
            }),
        })
    }
}

impl Default for NewToolDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ── AnomalyDetector (orchestrator) ────────────────────────────────────────────

/// Orchestrates all anomaly detectors.
///
/// Add this as `Arc<Mutex<AnomalyDetector>>` to `AppState` / background writer.
/// Call `check(event)` for each event after DB flush — never in the hot path.
pub struct AnomalyDetector {
    pub off_hours: OffHoursDetector,
    pub token_volume: TokenVolumeDetector,
    pub new_tool: NewToolDetector,
}

impl AnomalyDetector {
    /// Create an orchestrator with all detectors using default configuration.
    pub fn new() -> Self {
        Self {
            off_hours: OffHoursDetector::new(),
            token_volume: TokenVolumeDetector::new(),
            new_tool: NewToolDetector::new(),
        }
    }

    /// Run all detectors against a single event.
    ///
    /// Returns a `Vec` (possibly empty) of alerts. Never panics.
    pub fn check(&mut self, event: &AgentEvent) -> Vec<AnomalyAlert> {
        let mut alerts = Vec::new();

        // Each detector is individually fail-safe: they return Option<>, not Result<>.
        if let Some(a) = self.off_hours.check(event) {
            alerts.push(a);
        }
        if let Some(a) = self.token_volume.check(event) {
            alerts.push(a);
        }
        if let Some(a) = self.new_tool.check(event) {
            alerts.push(a);
        }

        alerts
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use govrix_scout_common::models::event::{EventDirection, Provider};

    /// Build a minimal AgentEvent for testing.
    fn make_event(agent_id: &str) -> AgentEvent {
        AgentEvent::new(
            agent_id,
            uuid::Uuid::nil(),
            EventDirection::Outbound,
            "POST",
            "https://test.example.com",
            Provider::Mcp,
            "genesis",
            "audit:none",
        )
    }

    // ── OffHoursDetector ─────────────────────────────────────────────────────

    #[test]
    fn off_hours_detector_triggers_at_night() {
        let detector = OffHoursDetector::new();
        let mut event = make_event("agent1");
        // 3 AM UTC — outside business hours
        event.timestamp = Utc.with_ymd_and_hms(2026, 3, 15, 3, 0, 0).unwrap();
        assert!(detector.check(&event).is_some());
    }

    #[test]
    fn off_hours_detector_triggers_at_midnight() {
        let detector = OffHoursDetector::new();
        let mut event = make_event("agent1");
        event.timestamp = Utc.with_ymd_and_hms(2026, 3, 15, 0, 0, 0).unwrap();
        assert!(detector.check(&event).is_some());
    }

    #[test]
    fn off_hours_detector_allows_business_hours() {
        let detector = OffHoursDetector::new();
        let mut event = make_event("agent1");
        // 10 AM UTC — inside business hours
        event.timestamp = Utc.with_ymd_and_hms(2026, 3, 15, 10, 0, 0).unwrap();
        assert!(detector.check(&event).is_none());
    }

    #[test]
    fn off_hours_detector_boundary_at_start() {
        let detector = OffHoursDetector::new();
        let mut event = make_event("agent1");
        // Exactly 6 AM — inside (inclusive start)
        event.timestamp = Utc.with_ymd_and_hms(2026, 3, 15, 6, 0, 0).unwrap();
        assert!(detector.check(&event).is_none());
    }

    #[test]
    fn off_hours_detector_boundary_at_end() {
        let detector = OffHoursDetector::new();
        let mut event = make_event("agent1");
        // Exactly 22 — outside (exclusive end)
        event.timestamp = Utc.with_ymd_and_hms(2026, 3, 15, 22, 0, 0).unwrap();
        assert!(detector.check(&event).is_some());
    }

    // ── TokenVolumeDetector ──────────────────────────────────────────────────

    #[test]
    fn token_volume_no_alert_during_warmup() {
        let mut detector = TokenVolumeDetector::new();
        let mut event = make_event("agent1");
        event.input_tokens = Some(10000); // very high but in warm-up

        // Warm-up period: first 10 events
        for _ in 0..10 {
            assert!(detector.check(&event).is_none());
        }
    }

    #[test]
    fn token_volume_no_alert_for_normal_spike() {
        let mut detector = TokenVolumeDetector::new();
        let mut event = make_event("agent1");
        event.input_tokens = Some(100);
        event.output_tokens = Some(100);

        // Build up an EMA with 10 warm-up events of 200 tokens each
        for _ in 0..10 {
            detector.check(&event);
        }

        // After warm-up, 300 tokens (1.5× EMA) should NOT trigger
        event.input_tokens = Some(150);
        event.output_tokens = Some(150);
        assert!(detector.check(&event).is_none());
    }

    #[test]
    fn token_volume_alert_on_large_spike() {
        let mut detector = TokenVolumeDetector::new();
        let mut event = make_event("agent1");
        event.input_tokens = Some(100);
        event.output_tokens = Some(100);

        // Build up EMA with warm-up events (200 tokens each)
        for _ in 0..11 {
            detector.check(&event);
        }

        // 10,000 tokens (50× EMA of ~200) should trigger
        event.input_tokens = Some(5000);
        event.output_tokens = Some(5000);
        let alert = detector.check(&event);
        assert!(alert.is_some());
        let a = alert.unwrap();
        assert_eq!(a.anomaly_type, AnomalyType::UnusualTokenVolume);
        assert_eq!(a.severity, AnomalySeverity::Warning);
    }

    #[test]
    fn token_volume_skips_zero_token_events() {
        let mut detector = TokenVolumeDetector::new();
        let event = make_event("agent1"); // tokens default to None (0)
        assert!(detector.check(&event).is_none());
    }

    // ── NewToolDetector ──────────────────────────────────────────────────────

    #[test]
    fn new_tool_detector_no_alert_during_warmup() {
        let mut detector = NewToolDetector::new();
        let mut event = make_event("agent1");
        event.event_kind = EventKind::ToolInvoke;

        for tool in &["tool_a", "tool_b", "tool_c"] {
            event.tool_name = Some(tool.to_string());
            assert!(
                detector.check(&event).is_none(),
                "warmup tool should not alert"
            );
        }
    }

    #[test]
    fn new_tool_detector_alerts_after_warmup() {
        let mut detector = NewToolDetector::new();
        let mut event = make_event("agent1");
        event.event_kind = EventKind::ToolInvoke;

        // Warm-up: 3 tools
        for tool in &["tool_a", "tool_b", "tool_c"] {
            event.tool_name = Some(tool.to_string());
            detector.check(&event);
        }

        // 4th new tool should alert
        event.tool_name = Some("suspicious_new_tool".to_string());
        let alert = detector.check(&event);
        assert!(alert.is_some());
        let a = alert.unwrap();
        assert_eq!(a.anomaly_type, AnomalyType::NewToolFirstSeen);
    }

    #[test]
    fn new_tool_detector_no_alert_for_repeat_tool() {
        let mut detector = NewToolDetector::new();
        let mut event = make_event("agent1");
        event.event_kind = EventKind::ToolInvoke;

        // First 3 warm-up tools
        for tool in &["a", "b", "c"] {
            event.tool_name = Some(tool.to_string());
            detector.check(&event);
        }

        // New tool (alerts once)
        event.tool_name = Some("new_tool".to_string());
        let first = detector.check(&event);
        assert!(first.is_some());

        // Same tool again — no alert
        let second = detector.check(&event);
        assert!(second.is_none());
    }

    #[test]
    fn new_tool_detector_ignores_non_invoke_events() {
        let mut detector = NewToolDetector::new();
        let mut event = make_event("agent1");
        event.event_kind = EventKind::LlmRequest; // NOT tool.invoke
        event.tool_name = Some("some_tool".to_string());

        assert!(detector.check(&event).is_none());
    }

    // ── AnomalyDetector (orchestrator) ───────────────────────────────────────

    #[test]
    fn orchestrator_returns_off_hours_alert() {
        let mut detector = AnomalyDetector::new();
        let mut event = make_event("agent1");
        event.timestamp = Utc.with_ymd_and_hms(2026, 3, 15, 2, 0, 0).unwrap();
        let alerts = detector.check(&event);
        assert!(!alerts.is_empty());
        assert!(alerts
            .iter()
            .any(|a| a.anomaly_type == AnomalyType::OffHoursUsage));
    }

    #[test]
    fn orchestrator_returns_empty_for_normal_event() {
        let mut detector = AnomalyDetector::new();
        let mut event = make_event("agent1");
        // Business hours, no tokens, no tool
        event.timestamp = Utc.with_ymd_and_hms(2026, 3, 15, 10, 0, 0).unwrap();
        let alerts = detector.check(&event);
        assert!(alerts.is_empty());
    }
}
