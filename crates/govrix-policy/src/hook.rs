//! Bridge between Scout's [`PolicyHook`] trait and the Govrix [`PolicyEngine`].
//!
//! [`GovrixPolicyHook`] wraps the platform's policy engine so it can be
//! injected into the Scout proxy interceptor, providing compliance tagging
//! and request blocking via the Govrix rule set.

use agentmesh_common::models::event::AgentEvent;
use agentmesh_proxy::policy::PolicyHook;

use crate::engine::{PolicyDecision, PolicyEngine};
use crate::pii;

/// Bridges Scout's `PolicyHook` trait to the Govrix `PolicyEngine`.
///
/// When `pii_enabled` is true, the hook also scans event payloads for PII
/// and elevates the compliance tag to `"warn:pii-detected"` if found.
pub struct GovrixPolicyHook {
    engine: PolicyEngine,
    pii_enabled: bool,
}

impl GovrixPolicyHook {
    /// Create a new hook wrapping the given engine.
    pub fn new(engine: PolicyEngine, pii_enabled: bool) -> Self {
        Self {
            engine,
            pii_enabled,
        }
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
}

impl PolicyHook for GovrixPolicyHook {
    fn compliance_tag(&self, event: &AgentEvent) -> String {
        // Engine rules take priority
        let engine_tag = match self.engine.evaluate(event) {
            PolicyDecision::Allow => "pass:all".to_string(),
            PolicyDecision::Block { reason } => return format!("block:{reason}"),
            PolicyDecision::Alert { message } => format!("warn:{message}"),
        };

        // PII check can elevate from pass to warn
        if self.has_pii(event) {
            return "warn:pii-detected".to_string();
        }

        engine_tag
    }

    fn check_request(&self, event: &AgentEvent) -> Option<String> {
        match self.engine.evaluate(event) {
            PolicyDecision::Block { reason } => Some(reason),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn empty_engine_returns_pass_all() {
        let hook = GovrixPolicyHook::new(PolicyEngine::new(), false);
        let event = make_event();
        assert_eq!(hook.compliance_tag(&event), "pass:all");
        assert_eq!(hook.check_request(&event), None);
    }

    #[test]
    fn blocking_rule_returns_block_tag() {
        let mut engine = PolicyEngine::new();
        engine.load_rules(vec![PolicyRule {
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
        let mut engine = PolicyEngine::new();
        engine.load_rules(vec![PolicyRule {
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
        let hook = GovrixPolicyHook::new(PolicyEngine::new(), true);
        let mut event = make_event();
        event.tags = serde_json::json!({"content": "email me at alice@example.com"});
        assert_eq!(hook.compliance_tag(&event), "warn:pii-detected");
    }

    #[test]
    fn pii_disabled_does_not_scan() {
        let hook = GovrixPolicyHook::new(PolicyEngine::new(), false);
        let mut event = make_event();
        event.tags = serde_json::json!({"content": "email me at alice@example.com"});
        // PII disabled → no scan → pass:all
        assert_eq!(hook.compliance_tag(&event), "pass:all");
    }

    #[test]
    fn block_takes_priority_over_pii() {
        let mut engine = PolicyEngine::new();
        engine.load_rules(vec![PolicyRule {
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
}
