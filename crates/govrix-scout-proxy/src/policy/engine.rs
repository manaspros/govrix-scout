//! Policy engine — evaluates all enabled policies against an intercepted event.
//!
//! The `PolicyEngine` is constructed once at startup and shared (via `Arc`) across
//! all proxy handler tasks. Evaluation is synchronous and returns immediately
//! (no I/O, no blocking). The caller is responsible for running evaluation in a
//! `tokio::spawn` fire-and-forget task to keep the hot path latency-free.
//!
//! Compliance-first invariant:
//! Every decision carries a `compliance_tag` in the format `"{status}:{policy_name}"`.
//!
//! Fail-open:
//! If policy evaluation encounters any internal error, the function logs a warning
//! and returns a single `"audit:error"` decision so traffic is never blocked by
//! an engine fault.

#![allow(dead_code)]

use std::path::Path;
use std::sync::{Arc, Mutex};

use govrix_scout_common::models::event::AgentEvent;

use super::budget::BudgetTracker;
use super::loader::{self, PolicyConfig};
use super::pii::PiiDetector;
use super::types::{AlertSeverity, PolicyAction, PolicyDecision};

// ── PolicyEngine ──────────────────────────────────────────────────────────────

/// The main policy evaluation engine.
///
/// Holds all loaded policies and evaluates them in order against each event.
/// Returns `Vec<PolicyDecision>` — one entry per matched rule. An empty vec
/// means no policies matched (traffic is allowed implicitly).
pub struct PolicyEngine {
    /// Parsed policy configuration.
    config: PolicyConfig,
    /// PII detector (compiled regexes).
    pii_detector: PiiDetector,
    /// Budget tracker (in-memory daily usage, behind Mutex for shared access).
    budget_tracker: Arc<Mutex<BudgetTracker>>,
}

impl PolicyEngine {
    /// Build a `PolicyEngine` from a YAML policy file.
    ///
    /// If the file does not exist, returns an engine with no policies (allow-all).
    /// Fails only if the file exists but cannot be parsed.
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let config = loader::load_from_file(path)?;
        Ok(Self::from_config(config))
    }

    /// Build a `PolicyEngine` from a YAML string (useful for tests).
    pub fn from_yaml(yaml: &str) -> anyhow::Result<Self> {
        let config = loader::load_from_str(yaml)?;
        Ok(Self::from_config(config))
    }

    /// Build a `PolicyEngine` from a pre-loaded `PolicyConfig`.
    pub fn from_config(config: PolicyConfig) -> Self {
        let budget_policy = loader::extract_budget_policy(&config);
        let budget_tracker = Arc::new(Mutex::new(BudgetTracker::new(budget_policy)));
        Self {
            config,
            pii_detector: PiiDetector::new(),
            budget_tracker,
        }
    }

    /// Create an engine with no policies (allow-all, used in testing).
    pub fn noop() -> Self {
        Self::from_config(PolicyConfig::default())
    }

    /// Evaluate all enabled policies against `event`.
    ///
    /// Returns a `Vec<PolicyDecision>` with one entry per triggered rule.
    /// An empty vec means "allow — no policies triggered".
    ///
    /// This function is:
    /// - Synchronous and fast (no I/O, no blocking)
    /// - Fail-open: panics are caught and converted to `"audit:error"`
    /// - Non-blocking: callers should `tokio::spawn` this for hot-path safety
    pub fn evaluate(&self, event: &AgentEvent) -> Vec<PolicyDecision> {
        // Catch any internal panic — fail-open
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.evaluate_inner(event)))
        {
            Ok(decisions) => decisions,
            Err(_) => {
                tracing::warn!(
                    agent_id = %event.agent_id,
                    event_id = %event.id,
                    "policy engine panicked during evaluation — failing open"
                );
                vec![PolicyDecision {
                    rule_id: "engine-error".to_string(),
                    action: PolicyAction::Allow,
                    timestamp: chrono::Utc::now(),
                    compliance_tag: "audit:error".to_string(),
                }]
            }
        }
    }

    /// Compute the single highest-severity compliance tag from a list of decisions.
    ///
    /// Used by the interceptor to set `event.compliance_tag`.
    /// Format: `"{status}:{rule_id}"` per compliance-first skill.
    ///
    /// Returns `"audit:none"` if the decisions list is empty.
    /// Returns `"pass:all"` if all decisions are `Allow`.
    pub fn compute_compliance_tag(decisions: &[PolicyDecision]) -> String {
        if decisions.is_empty() {
            return "audit:none".to_string();
        }

        match PolicyDecision::worst(decisions) {
            None => "audit:none".to_string(),
            Some(worst) => {
                if worst.action == PolicyAction::Allow {
                    "pass:all".to_string()
                } else {
                    worst.compliance_tag.clone()
                }
            }
        }
    }

    // ── Private evaluation ────────────────────────────────────────────────────

    fn evaluate_inner(&self, event: &AgentEvent) -> Vec<PolicyDecision> {
        let mut decisions = Vec::new();

        for entry in &self.config.policies {
            if !entry.enabled {
                continue;
            }

            // Dispatch to per-policy evaluator based on ID prefix or rule content
            let is_pii =
                entry.id.contains("pii") || entry.rules.iter().any(|r| !r.scan_for.is_empty());
            let is_budget = entry.id.contains("budget")
                || entry
                    .rules
                    .iter()
                    .any(|r| r.daily_token_limit.is_some() || r.daily_cost_limit_usd.is_some());

            if is_pii {
                let mut pii_decisions = self.evaluate_pii_policy(entry, event);
                decisions.append(&mut pii_decisions);
            }

            if is_budget {
                let mut budget_decisions = self.evaluate_budget_policy(entry, event);
                decisions.append(&mut budget_decisions);
            }
        }

        decisions
    }

    /// Evaluate a PII detection policy against the event payload.
    fn evaluate_pii_policy(
        &self,
        entry: &loader::PolicyEntry,
        event: &AgentEvent,
    ) -> Vec<PolicyDecision> {
        let mut decisions = Vec::new();

        // Collect all scan_for types across all rules in this policy
        let scan_types: Vec<String> = entry
            .rules
            .iter()
            .flat_map(|r| r.scan_for.iter().cloned())
            .collect();

        if scan_types.is_empty() {
            return decisions;
        }

        // Scan the event payload for PII
        let payload = match &event.payload {
            Some(p) => p.clone(),
            None => {
                // Also check tags for any string content
                event.tags.clone()
            }
        };

        let matches = self.pii_detector.scan_json(&payload, "payload");

        if matches.is_empty() {
            return decisions;
        }

        // Filter matches to only the scan_for types in this policy
        let relevant_matches: Vec<_> = matches
            .iter()
            .filter(|m| {
                let type_str = format!("{}", m.pii_type).to_lowercase();
                scan_types.iter().any(|t| {
                    t == &type_str
                        || (t == "credit_card" && type_str == "credit_card")
                        || (t == "ssn" && type_str == "ssn")
                        || (t == "ip_address" && type_str == "ip_address")
                })
            })
            .collect();

        if relevant_matches.is_empty() {
            return decisions;
        }

        // Determine the action from the rule
        let severity = entry
            .rules
            .first()
            .and_then(|r| r.severity.as_deref())
            .unwrap_or("high");

        let alert_severity = parse_severity(severity);

        let types_found: Vec<String> = relevant_matches
            .iter()
            .map(|m| format!("{}", m.pii_type))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let action = PolicyAction::Alert {
            message: format!(
                "PII detected in event for agent '{}': {} type(s) found in payload",
                event.agent_id,
                types_found.join(", ")
            ),
            severity: alert_severity,
        };

        decisions.push(PolicyDecision::new(&entry.id, action));

        decisions
    }

    /// Evaluate a budget policy against the event's token/cost data.
    fn evaluate_budget_policy(
        &self,
        _entry: &loader::PolicyEntry,
        event: &AgentEvent,
    ) -> Vec<PolicyDecision> {
        let mut decisions = Vec::new();

        let estimated_tokens = event.total_tokens.unwrap_or(0).max(0) as u64;
        let estimated_cost = event
            .cost_usd
            .map(|d| {
                use std::str::FromStr;
                f64::from_str(&d.to_string()).unwrap_or(0.0)
            })
            .unwrap_or(0.0);

        let action = {
            let tracker = match self.budget_tracker.lock() {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("budget tracker mutex poisoned: {}", e);
                    return decisions;
                }
            };
            tracker.check_budget(&event.agent_id, estimated_tokens, estimated_cost)
        };

        match &action {
            PolicyAction::Allow => {}
            _ => {
                decisions.push(PolicyDecision::new("budget-cap", action));
            }
        }

        decisions
    }

    /// Record actual usage after an event completes (call post-response).
    ///
    /// This updates the in-memory budget counters and is safe to call from
    /// a fire-and-forget tokio task.
    pub fn record_usage(&self, agent_id: &str, tokens: u64, cost_usd: f64) {
        if let Ok(mut tracker) = self.budget_tracker.lock() {
            tracker.record_usage(agent_id, tokens, cost_usd);
        } else {
            tracing::warn!(
                agent_id = %agent_id,
                "budget tracker mutex poisoned — usage not recorded"
            );
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_severity(s: &str) -> AlertSeverity {
    match s.to_lowercase().as_str() {
        "low" => AlertSeverity::Low,
        "medium" => AlertSeverity::Medium,
        "high" => AlertSeverity::High,
        "critical" => AlertSeverity::Critical,
        _ => AlertSeverity::High, // default to high for unknown values
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use govrix_scout_common::models::event::{AgentEvent, EventDirection, Provider};
    use uuid::Uuid;

    fn make_event(agent_id: &str) -> AgentEvent {
        AgentEvent::new(
            agent_id,
            Uuid::now_v7(),
            EventDirection::Outbound,
            "POST",
            "https://api.openai.com/v1/chat/completions",
            Provider::OpenAI,
            "GENESIS",
            "audit:none",
        )
    }

    // ── Noop engine ───────────────────────────────────────────────────────────

    #[test]
    fn noop_engine_returns_empty_decisions() {
        let engine = PolicyEngine::noop();
        let event = make_event("agent-1");
        let decisions = engine.evaluate(&event);
        assert!(decisions.is_empty());
    }

    // ── PII policy ────────────────────────────────────────────────────────────

    #[test]
    fn pii_policy_triggers_on_email_in_payload() {
        let yaml = r#"
policies:
  - id: pii-detection
    name: "PII Detection"
    enabled: true
    rules:
      - scan_for: [email]
        action: alert
        severity: high
"#;
        let engine = PolicyEngine::from_yaml(yaml).unwrap();
        let mut event = make_event("agent-1");
        event.payload = Some(serde_json::json!({
            "messages": [{"role": "user", "content": "My email is alice@example.com"}]
        }));
        let decisions = engine.evaluate(&event);
        assert!(!decisions.is_empty());
        assert!(decisions[0].compliance_tag.contains("pii-detection"));
    }

    #[test]
    fn pii_policy_no_trigger_on_clean_payload() {
        let yaml = r#"
policies:
  - id: pii-detection
    name: "PII Detection"
    enabled: true
    rules:
      - scan_for: [email, phone]
        action: alert
        severity: high
"#;
        let engine = PolicyEngine::from_yaml(yaml).unwrap();
        let mut event = make_event("agent-1");
        event.payload = Some(serde_json::json!({
            "messages": [{"role": "user", "content": "What is the capital of France?"}]
        }));
        let decisions = engine.evaluate(&event);
        // No PII → no decisions
        assert!(decisions.is_empty());
    }

    #[test]
    fn disabled_pii_policy_does_not_trigger() {
        let yaml = r#"
policies:
  - id: pii-detection
    name: "PII Detection"
    enabled: false
    rules:
      - scan_for: [email]
        action: alert
        severity: high
"#;
        let engine = PolicyEngine::from_yaml(yaml).unwrap();
        let mut event = make_event("agent-1");
        event.payload = Some(serde_json::json!({
            "text": "Email alice@example.com"
        }));
        let decisions = engine.evaluate(&event);
        assert!(decisions.is_empty());
    }

    // ── Budget policy ─────────────────────────────────────────────────────────

    #[test]
    fn budget_policy_allows_within_limit() {
        let yaml = r#"
policies:
  - id: budget-cap
    name: "Daily Budget Cap"
    enabled: true
    rules:
      - agent_class: "*"
        daily_cost_limit_usd: 100.0
        daily_token_limit: 1000000
        action: block
"#;
        let engine = PolicyEngine::from_yaml(yaml).unwrap();
        let mut event = make_event("agent-1");
        event.total_tokens = Some(1000);
        let decisions = engine.evaluate(&event);
        // Under limit → no block decisions
        assert!(!decisions
            .iter()
            .any(|d| matches!(d.action, PolicyAction::Block { .. })));
    }

    #[test]
    fn budget_policy_blocks_when_over_limit() {
        let yaml = r#"
policies:
  - id: budget-cap
    name: "Daily Budget Cap"
    enabled: true
    rules:
      - agent_class: "*"
        daily_token_limit: 10
        action: block
"#;
        let engine = PolicyEngine::from_yaml(yaml).unwrap();

        // Pre-fill usage to be near limit
        engine.record_usage("agent-over", 9, 0.0);

        let mut event = make_event("agent-over");
        event.total_tokens = Some(5); // 9 + 5 = 14 > 10 → block

        let decisions = engine.evaluate(&event);
        assert!(decisions
            .iter()
            .any(|d| matches!(d.action, PolicyAction::Block { .. })));
    }

    // ── Compliance tag computation ─────────────────────────────────────────────

    #[test]
    fn compliance_tag_is_audit_none_when_no_decisions() {
        let tag = PolicyEngine::compute_compliance_tag(&[]);
        assert_eq!(tag, "audit:none");
    }

    #[test]
    fn compliance_tag_is_pass_all_when_only_allow() {
        let d = PolicyDecision::new("some-rule", PolicyAction::Allow);
        let tag = PolicyEngine::compute_compliance_tag(&[d]);
        assert_eq!(tag, "pass:all");
    }

    #[test]
    fn compliance_tag_shows_worst_decision() {
        let allow = PolicyDecision::new("rule-1", PolicyAction::Allow);
        let warn = PolicyDecision::new(
            "pii-detection",
            PolicyAction::Alert {
                message: "PII found".into(),
                severity: AlertSeverity::High,
            },
        );
        let tag = PolicyEngine::compute_compliance_tag(&[allow, warn]);
        assert_eq!(tag, "warn:pii-detection");
    }

    #[test]
    fn compliance_tag_block_takes_precedence_over_warn() {
        let warn = PolicyDecision::new(
            "pii-detection",
            PolicyAction::Alert {
                message: "PII found".into(),
                severity: AlertSeverity::High,
            },
        );
        let block = PolicyDecision::new(
            "budget-cap",
            PolicyAction::Block {
                reason: "over limit".into(),
            },
        );
        let tag = PolicyEngine::compute_compliance_tag(&[warn, block]);
        assert_eq!(tag, "block:budget-cap");
    }

    // ── record_usage ──────────────────────────────────────────────────────────

    #[test]
    fn record_usage_persists_across_evaluations() {
        let yaml = r#"
policies:
  - id: budget-cap
    name: "Budget"
    enabled: true
    rules:
      - agent_class: "agent-x"
        daily_token_limit: 100
        action: block
"#;
        let engine = PolicyEngine::from_yaml(yaml).unwrap();
        engine.record_usage("agent-x", 90, 0.0);

        let mut event = make_event("agent-x");
        event.total_tokens = Some(20); // 90 + 20 > 100

        let decisions = engine.evaluate(&event);
        assert!(decisions
            .iter()
            .any(|d| matches!(d.action, PolicyAction::Block { .. })));
    }

    // ── parse_severity ────────────────────────────────────────────────────────

    #[test]
    fn parse_severity_variants() {
        assert_eq!(parse_severity("low"), AlertSeverity::Low);
        assert_eq!(parse_severity("medium"), AlertSeverity::Medium);
        assert_eq!(parse_severity("high"), AlertSeverity::High);
        assert_eq!(parse_severity("critical"), AlertSeverity::Critical);
        assert_eq!(parse_severity("UNKNOWN"), AlertSeverity::High);
    }
}
