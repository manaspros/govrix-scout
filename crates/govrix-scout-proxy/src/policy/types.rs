//! Policy types — core enums and structs for the policy engine.
//!
//! These types represent the building blocks of policy rules, conditions,
//! actions, and decisions. Every `PolicyDecision` carries a `compliance_tag`
//! per the compliance-first skill invariant.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Actions ───────────────────────────────────────────────────────────────────

/// What the policy engine should do when a rule fires.
///
/// In OSS, `Block` and `Redact` are advisory (logged only, traffic is not stopped).
/// In the SaaS policy engine these become enforcement actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyAction {
    /// Traffic is compliant — allow through.
    Allow,

    /// Hard violation — block the request (SaaS enforcement; logged advisory in OSS).
    Block {
        /// Human-readable reason for the block.
        reason: String,
    },

    /// Sensitive fields should be redacted from stored events.
    ///
    /// `fields` is a list of JSON-path-style field names (e.g. `["messages[0].content"]`).
    Redact {
        /// Field paths to redact from the logged payload.
        fields: Vec<String>,
    },

    /// Non-blocking alert — emit a compliance warning.
    Alert {
        /// Human-readable alert message.
        message: String,
        /// How severe this alert is.
        severity: AlertSeverity,
    },
}

impl PolicyAction {
    /// Returns the compliance status string for the compliance_tag field.
    ///
    /// Follows the compliance-first skill tag format: `{status}:{policy_name}`.
    pub fn compliance_status(&self) -> &'static str {
        match self {
            PolicyAction::Allow => "pass",
            PolicyAction::Block { .. } => "block",
            PolicyAction::Redact { .. } => "warn",
            PolicyAction::Alert { severity, .. } => match severity {
                AlertSeverity::Low => "pass",
                AlertSeverity::Medium => "warn",
                AlertSeverity::High => "warn",
                AlertSeverity::Critical => "block",
            },
        }
    }
}

// ── Severity ──────────────────────────────────────────────────────────────────

/// Alert severity levels, ordered from least to most severe.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Low => write!(f, "low"),
            AlertSeverity::Medium => write!(f, "medium"),
            AlertSeverity::High => write!(f, "high"),
            AlertSeverity::Critical => write!(f, "critical"),
        }
    }
}

// ── Conditions ────────────────────────────────────────────────────────────────

/// A single condition within a policy rule.
///
/// The condition is evaluated against a specific `field` in the event
/// using the given `operator` and expected `value`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCondition {
    /// Dot-notation path to the event field (e.g. `"agent_id"`, `"total_tokens"`).
    pub field: String,
    /// The comparison operator.
    pub operator: ConditionOperator,
    /// The value to compare against (as a JSON value for type flexibility).
    pub value: serde_json::Value,
}

/// Comparison operators for policy conditions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    /// Field value equals `value`.
    Equals,
    /// Field value does not equal `value`.
    NotEquals,
    /// Field value (as string) contains `value` (as string).
    Contains,
    /// Field value (as string) matches `value` (as regex pattern).
    Regex,
    /// Field value (numeric) is greater than `value`.
    GreaterThan,
    /// Field value (numeric) is less than `value`.
    LessThan,
}

// ── Rules ─────────────────────────────────────────────────────────────────────

/// A single policy rule: if all `conditions` match, apply `action`.
///
/// Rules are evaluated in order within a policy. The first matching rule fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Unique identifier for this rule (used in compliance_tag).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Explanation of what this rule enforces.
    pub description: String,
    /// If `false`, this rule is skipped during evaluation.
    pub enabled: bool,
    /// All conditions must be true for the rule to fire (AND semantics).
    pub conditions: Vec<PolicyCondition>,
    /// Action to take when the rule fires.
    pub action: PolicyAction,
}

// ── Decisions ─────────────────────────────────────────────────────────────────

/// The outcome of evaluating a policy rule against an event.
///
/// Every `PolicyDecision` includes the four compliance fields required by
/// the compliance-first skill: `rule_id` maps to the policy name in the
/// `compliance_tag`, `timestamp` is UTC, and `compliance_tag` is formatted
/// as `"{status}:{policy_name}"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// ID of the rule that produced this decision.
    pub rule_id: String,
    /// The action the policy engine recommends.
    pub action: PolicyAction,
    /// UTC timestamp of when this decision was made (compliance field).
    pub timestamp: DateTime<Utc>,
    /// Compliance tag: `"{status}:{rule_name}"` (compliance-first invariant).
    ///
    /// Examples:
    /// - `"pass:all"`
    /// - `"warn:pii-detection"`
    /// - `"block:budget-cap"`
    pub compliance_tag: String,
}

impl PolicyDecision {
    /// Create a new decision with an automatically-computed compliance tag.
    ///
    /// Tag format: `"{action_status}:{rule_id}"` per compliance-first skill.
    pub fn new(rule_id: impl Into<String>, action: PolicyAction) -> Self {
        let rule_id = rule_id.into();
        let status = action.compliance_status();
        let compliance_tag = format!("{}:{}", status, rule_id);
        Self {
            rule_id,
            action,
            timestamp: Utc::now(),
            compliance_tag,
        }
    }

    /// Returns the highest-severity decision from a slice of decisions.
    ///
    /// Priority: Block > Redact/Alert(Critical) > Alert(High) > Alert(Medium) > Allow
    pub fn worst(decisions: &[PolicyDecision]) -> Option<&PolicyDecision> {
        decisions.iter().max_by_key(|d| match &d.action {
            PolicyAction::Allow => 0u8,
            PolicyAction::Alert { severity, .. } => match severity {
                AlertSeverity::Low => 1,
                AlertSeverity::Medium => 2,
                AlertSeverity::High => 3,
                AlertSeverity::Critical => 4,
            },
            PolicyAction::Redact { .. } => 4,
            PolicyAction::Block { .. } => 5,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_action_compliance_status() {
        assert_eq!(PolicyAction::Allow.compliance_status(), "pass");
        assert_eq!(
            PolicyAction::Block {
                reason: "test".into()
            }
            .compliance_status(),
            "block"
        );
        assert_eq!(
            PolicyAction::Alert {
                message: "test".into(),
                severity: AlertSeverity::High
            }
            .compliance_status(),
            "warn"
        );
        assert_eq!(
            PolicyAction::Alert {
                message: "test".into(),
                severity: AlertSeverity::Critical
            }
            .compliance_status(),
            "block"
        );
    }

    #[test]
    fn policy_decision_compliance_tag_format() {
        let d = PolicyDecision::new(
            "pii-detection",
            PolicyAction::Alert {
                message: "PII found".into(),
                severity: AlertSeverity::High,
            },
        );
        assert_eq!(d.compliance_tag, "warn:pii-detection");
    }

    #[test]
    fn policy_decision_block_tag() {
        let d = PolicyDecision::new(
            "budget-cap",
            PolicyAction::Block {
                reason: "daily limit exceeded".into(),
            },
        );
        assert_eq!(d.compliance_tag, "block:budget-cap");
    }

    #[test]
    fn worst_decision_selects_block_over_allow() {
        let allow = PolicyDecision::new("r1", PolicyAction::Allow);
        let block = PolicyDecision::new(
            "r2",
            PolicyAction::Block {
                reason: "exceeded".into(),
            },
        );
        let decisions = vec![allow, block];
        let worst = PolicyDecision::worst(&decisions).unwrap();
        assert_eq!(worst.rule_id, "r2");
    }

    #[test]
    fn worst_decision_on_empty_slice_is_none() {
        assert!(PolicyDecision::worst(&[]).is_none());
    }

    #[test]
    fn alert_severity_ordering() {
        assert!(AlertSeverity::Critical > AlertSeverity::High);
        assert!(AlertSeverity::High > AlertSeverity::Medium);
        assert!(AlertSeverity::Medium > AlertSeverity::Low);
    }
}
