//! Bridge between Scout's [`PolicyHook`] trait and the Govrix [`PolicyEngine`].
//!
//! [`GovrixPolicyHook`] wraps the platform's policy engine so it can be
//! injected into the Scout proxy interceptor, providing compliance tagging
//! and request blocking via the Govrix rule set.
//!
//! The engine is stored behind `Arc<RwLock<PolicyEngine>>` so that it can be
//! shared with other components (e.g. the management API) and reloaded at
//! runtime without restarting the server.

use std::sync::{Arc, RwLock};

use agentmesh_common::models::event::AgentEvent;
use agentmesh_proxy::policy::PolicyHook;

use crate::budget::{BudgetResult, BudgetTracker};
use crate::engine::{PolicyDecision, PolicyEngine};
use crate::pii;

/// Bridges Scout's `PolicyHook` trait to the Govrix `PolicyEngine`.
///
/// When `pii_enabled` is true, the hook also scans event payloads for PII
/// and elevates the compliance tag to `"warn:pii-detected"` if found.
///
/// When a [`BudgetTracker`] is attached via [`GovrixPolicyHook::with_budget`],
/// budget checks are performed during every request evaluation. Requests from
/// agents that have already exceeded their budget are blocked.
pub struct GovrixPolicyHook {
    engine: Arc<RwLock<PolicyEngine>>,
    pii_enabled: bool,
    budget: Option<Arc<BudgetTracker>>,
}

impl GovrixPolicyHook {
    /// Create a new hook wrapping the given shared engine. Budget checking is
    /// disabled by default; attach one with [`with_budget`](Self::with_budget).
    pub fn new(engine: Arc<RwLock<PolicyEngine>>, pii_enabled: bool) -> Self {
        Self {
            engine,
            pii_enabled,
            budget: None,
        }
    }

    /// Attach a [`BudgetTracker`] to this hook (builder pattern).
    ///
    /// Once attached, every call to [`check_request`](PolicyHook::check_request)
    /// and [`compliance_tag`](PolicyHook::compliance_tag) will consult the
    /// tracker. Requests that would push an agent over its configured limit are
    /// blocked with a `warn:budget-exceeded` tag or a blocking reason string.
    pub fn with_budget(mut self, budget: Arc<BudgetTracker>) -> Self {
        self.budget = Some(budget);
        self
    }

    /// Check if an event payload contains PII.
    fn has_pii(&self, event: &AgentEvent) -> bool {
        if !self.pii_enabled {
            return false;
        }
        // Check the upstream URL for PII (unlikely but thorough)
        if pii::mask_pii(&event.upstream_target) != event.upstream_target {
            return true;
        }
        // Check tags (often contains user content)
        let tags_str = event.tags.to_string();
        pii::mask_pii(&tags_str) != tags_str
    }

    /// Check the budget tracker for the event's agent.
    ///
    /// Uses the event's `total_tokens` and `cost_usd` when available so that
    /// accumulated usage is recorded accurately. At request-interception time
    /// these fields are typically `None` (the response has not arrived yet), so
    /// the call records zero usage — which still catches agents that are
    /// *already* over budget from previous requests.
    fn check_budget(&self, event: &AgentEvent) -> Option<String> {
        let tracker = self.budget.as_ref()?;

        let tokens = event.total_tokens.map(|t| t.max(0) as u64).unwrap_or(0);

        let cost = event
            .cost_usd
            .map(|d| {
                use std::str::FromStr;
                f64::from_str(&d.to_string()).unwrap_or(0.0)
            })
            .unwrap_or(0.0);

        match tracker.check_and_record(&event.agent_id, tokens, cost) {
            BudgetResult::WithinBudget => None,
            BudgetResult::Exceeded { reason } => Some(reason),
        }
    }
}

impl PolicyHook for GovrixPolicyHook {
    fn compliance_tag(&self, event: &AgentEvent) -> String {
        // Engine rules take priority
        let engine_tag = match self.engine.read().unwrap().evaluate(event) {
            PolicyDecision::Allow => "pass:all".to_string(),
            PolicyDecision::Block { reason } => return format!("block:{reason}"),
            PolicyDecision::Alert { message } => format!("warn:{message}"),
        };

        // Budget check: exceeded budget elevates to warn
        if let Some(_reason) = self.check_budget(event) {
            return "warn:budget-exceeded".to_string();
        }

        // PII check can elevate from pass to warn
        if self.has_pii(event) {
            return "warn:pii-detected".to_string();
        }

        engine_tag
    }

    fn check_request(&self, event: &AgentEvent) -> Option<String> {
        if let PolicyDecision::Block { reason } = self.engine.read().unwrap().evaluate(event) {
            return Some(reason);
        }

        // Budget exceeded → block the request with the reason
        if let Some(reason) = self.check_budget(event) {
            return Some(reason);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::BudgetLimit;
    use crate::engine::{Action, Condition, Operator, PolicyRule};
    use agentmesh_common::models::event::{EventDirection, Provider};
    use uuid::Uuid;

    fn make_event() -> AgentEvent {
        AgentEvent::new(
            "agent-001",
            Uuid::now_v7(),
            EventDirection::Outbound,
            "POST",
            "https://api.openai.com/v1/chat/completions",
            Provider::OpenAI,
            "genesis",
            "audit:none",
        )
    }

    fn make_engine() -> Arc<RwLock<PolicyEngine>> {
        Arc::new(RwLock::new(PolicyEngine::new()))
    }

    #[test]
    fn empty_engine_returns_pass_all() {
        let hook = GovrixPolicyHook::new(make_engine(), false);
        let event = make_event();
        assert_eq!(hook.compliance_tag(&event), "pass:all");
        assert_eq!(hook.check_request(&event), None);
    }

    #[test]
    fn blocking_rule_returns_block_tag() {
        let engine = make_engine();
        engine.write().unwrap().load_rules(vec![PolicyRule {
            name: "no-openai".to_string(),
            description: None,
            enabled: true,
            conditions: vec![Condition {
                field: "provider".to_string(),
                operator: Operator::Equals,
                value: "openai".to_string(),
            }],
            action: Action::Block,
        }]);

        let hook = GovrixPolicyHook::new(engine, false);
        let event = make_event();

        assert_eq!(
            hook.compliance_tag(&event),
            "block:blocked by rule: no-openai"
        );
        assert_eq!(
            hook.check_request(&event),
            Some("blocked by rule: no-openai".to_string())
        );
    }

    #[test]
    fn alert_rule_returns_warn_tag_and_allows_request() {
        let engine = make_engine();
        engine.write().unwrap().load_rules(vec![PolicyRule {
            name: "alert-outbound".to_string(),
            description: None,
            enabled: true,
            conditions: vec![Condition {
                field: "direction".to_string(),
                operator: Operator::Equals,
                value: "outbound".to_string(),
            }],
            action: Action::Alert,
        }]);

        let hook = GovrixPolicyHook::new(engine, true);
        let event = make_event();

        assert_eq!(
            hook.compliance_tag(&event),
            "warn:alert from rule: alert-outbound"
        );
        // Alert does NOT block
        assert_eq!(hook.check_request(&event), None);
    }

    #[test]
    fn pii_detected_in_tags_elevates_to_warn() {
        let hook = GovrixPolicyHook::new(make_engine(), true);
        let mut event = make_event();
        event.tags = serde_json::json!({"content": "email me at alice@example.com"});
        assert_eq!(hook.compliance_tag(&event), "warn:pii-detected");
    }

    #[test]
    fn pii_disabled_does_not_scan() {
        let hook = GovrixPolicyHook::new(make_engine(), false);
        let mut event = make_event();
        event.tags = serde_json::json!({"content": "email me at alice@example.com"});
        // PII disabled → no scan → pass:all
        assert_eq!(hook.compliance_tag(&event), "pass:all");
    }

    #[test]
    fn block_takes_priority_over_pii() {
        let engine = make_engine();
        engine.write().unwrap().load_rules(vec![PolicyRule {
            name: "no-openai".to_string(),
            description: None,
            enabled: true,
            conditions: vec![Condition {
                field: "provider".to_string(),
                operator: Operator::Equals,
                value: "openai".to_string(),
            }],
            action: Action::Block,
        }]);

        let hook = GovrixPolicyHook::new(engine, true);
        let mut event = make_event();
        event.tags = serde_json::json!({"content": "email alice@example.com"});

        // Block should take priority over PII warn
        assert!(hook.compliance_tag(&event).starts_with("block:"));
    }

    // ── BudgetTracker integration tests ──────────────────────────────────────

    fn make_tracker_with_agent_limit(agent_id: &str, max_tokens: u64) -> Arc<BudgetTracker> {
        let mut tracker = BudgetTracker::new();
        tracker.set_agent_limit(
            agent_id.to_string(),
            BudgetLimit {
                max_tokens: Some(max_tokens),
                max_cost_usd: None,
            },
        );
        Arc::new(tracker)
    }

    /// Budget within limit: compliance_tag returns pass:all, check_request returns None.
    #[test]
    fn budget_within_limit_passes() {
        let tracker = make_tracker_with_agent_limit("agent-001", 10_000);
        let hook = GovrixPolicyHook::new(make_engine(), false).with_budget(tracker);
        let event = make_event();

        assert_eq!(hook.compliance_tag(&event), "pass:all");
        // check_request evaluates the budget a second time (still within limit)
        assert_eq!(hook.check_request(&event), None);
    }

    /// Budget already exhausted before request: compliance_tag returns warn:budget-exceeded
    /// and check_request returns Some(reason).
    #[test]
    fn budget_exceeded_blocks_request() {
        let mut tracker = BudgetTracker::new();
        tracker.set_agent_limit(
            "agent-001".to_string(),
            BudgetLimit {
                max_tokens: Some(100),
                max_cost_usd: None,
            },
        );
        let tracker = Arc::new(tracker);

        // Use exactly the full budget (100 tokens → at limit, still WithinBudget and recorded).
        tracker.check_and_record("agent-001", 100, 0.0);

        let hook = GovrixPolicyHook::new(make_engine(), false).with_budget(tracker);
        // The event carries 1 token; the hook will call check_and_record("agent-001", 1, 0.0)
        // which evaluates 100 + 1 = 101 > 100 → Exceeded.
        let mut event = make_event();
        event.total_tokens = Some(1);

        assert_eq!(hook.compliance_tag(&event), "warn:budget-exceeded");

        let block_reason = hook.check_request(&event);
        assert!(block_reason.is_some(), "expected Some(reason) but got None");
        let reason = block_reason.unwrap();
        assert!(
            reason.contains("agent-001"),
            "reason should mention the agent: {reason}"
        );
    }

    /// Budget disabled (None): existing behavior is unchanged.
    #[test]
    fn no_budget_no_change_to_existing_behavior() {
        // Hook without any budget attached
        let hook = GovrixPolicyHook::new(make_engine(), false);
        let event = make_event();

        assert_eq!(hook.compliance_tag(&event), "pass:all");
        assert_eq!(hook.check_request(&event), None);
    }

    /// Engine block takes priority over budget exceeded.
    #[test]
    fn engine_block_takes_priority_over_budget() {
        let engine = make_engine();
        engine.write().unwrap().load_rules(vec![PolicyRule {
            name: "no-openai".to_string(),
            description: None,
            enabled: true,
            conditions: vec![Condition {
                field: "provider".to_string(),
                operator: Operator::Equals,
                value: "openai".to_string(),
            }],
            action: Action::Block,
        }]);

        let mut tracker = BudgetTracker::new();
        tracker.set_agent_limit(
            "agent-001".to_string(),
            BudgetLimit {
                max_tokens: Some(100),
                max_cost_usd: None,
            },
        );
        let tracker = Arc::new(tracker);
        // Fill budget to 100, then the event carrying 1 token would exceed
        tracker.check_and_record("agent-001", 100, 0.0);

        let hook = GovrixPolicyHook::new(engine, false).with_budget(tracker);
        let mut event = make_event();
        event.total_tokens = Some(1);

        // Engine block should win over budget exceeded
        assert!(hook.compliance_tag(&event).starts_with("block:"));
        assert!(hook.check_request(&event).unwrap().contains("no-openai"));
    }
}
