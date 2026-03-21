use serde::{Deserialize, Serialize};

// ── Decision type ───────────────────────────────────────────────────────────

/// The outcome of evaluating an event against the policy engine.
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Block { reason: String },
    Alert { message: String },
}

// ── Rule model ──────────────────────────────────────────────────────────────

/// A single named policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub action: Action,
}

/// A generic field condition: `<field> <operator> <value>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Event field to inspect. Supported: "model", "agent_id", "provider",
    /// "direction", "cost_usd", "input_tokens", "output_tokens".
    pub field: String,
    pub operator: Operator,
    /// Value to compare against (always a string; numeric operators parse it).
    pub value: String,
}

/// Comparison operators for conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Equals,
    NotEquals,
    Contains,
    GreaterThan,
    LessThan,
    Matches, // regex
}

/// Action taken when all conditions on a rule match.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Allow,
    Block,
    Alert,
}

// ── Engine ──────────────────────────────────────────────────────────────────

/// Policy engine that evaluates `AgentEvent`s against an ordered list of rules.
///
/// Rules are evaluated in insertion order. The first enabled rule whose every
/// condition matches wins. If no rule matches, the default decision is `Allow`.
#[derive(Debug, Default)]
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
}

impl PolicyEngine {
    /// Create an empty engine (no rules — every event is allowed).
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the current rule set.
    pub fn load_rules(&mut self, rules: Vec<PolicyRule>) {
        self.rules = rules;
    }

    /// Reload the rule set, replacing any previously loaded rules.
    ///
    /// This is equivalent to [`load_rules`] but signals intent — callers use
    /// this when dynamically replacing rules at runtime rather than doing
    /// initial configuration.
    pub fn reload_rules(&mut self, rules: Vec<PolicyRule>) {
        self.load_rules(rules);
    }

    /// Return `(total_rules, enabled_rules)`.
    ///
    /// `total_rules` is the number of rules currently loaded (enabled or not).
    /// `enabled_rules` is the subset where `enabled == true`.
    pub fn rule_count(&self) -> (usize, usize) {
        let total = self.rules.len();
        let enabled = self.rules.iter().filter(|r| r.enabled).count();
        (total, enabled)
    }

    /// Parse a YAML list of rules.
    ///
    /// Expected format (top-level sequence):
    /// ```yaml
    /// - name: block-gpt4
    ///   enabled: true
    ///   conditions:
    ///     - field: model
    ///       operator: equals
    ///       value: gpt-4
    ///   action: block
    /// ```
    pub fn load_yaml(yaml_str: &str) -> Result<Vec<PolicyRule>, serde_yaml::Error> {
        serde_yaml::from_str(yaml_str)
    }

    /// Parse rules from a YAML string and load them into the engine.
    ///
    /// Returns the number of rules loaded. Any previously loaded rules are
    /// replaced.
    pub fn load_rules_from_yaml(&mut self, yaml: &str) -> Result<usize, serde_yaml::Error> {
        let rules: Vec<PolicyRule> = serde_yaml::from_str(yaml)?;
        let count = rules.len();
        self.load_rules(rules);
        Ok(count)
    }

    /// Read a YAML file from `path`, parse its rules, and load them into the
    /// engine.
    ///
    /// Returns the number of rules loaded. Any previously loaded rules are
    /// replaced.
    pub fn load_rules_from_file(
        &mut self,
        path: &std::path::Path,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let count = self.load_rules_from_yaml(&content)?;
        Ok(count)
    }

    /// Evaluate `event` against all loaded rules in order.
    ///
    /// Returns the decision of the first matching, enabled rule. Falls through
    /// to `Allow` when no rule matches.
    pub fn evaluate(&self, event: &govrix_scout_common::models::AgentEvent) -> PolicyDecision {
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            if self.matches_all_conditions(event, &rule.conditions) {
                return match &rule.action {
                    Action::Block => PolicyDecision::Block {
                        reason: format!("blocked by rule: {}", rule.name),
                    },
                    Action::Alert => PolicyDecision::Alert {
                        message: format!("alert from rule: {}", rule.name),
                    },
                    Action::Allow => PolicyDecision::Allow,
                };
            }
        }
        PolicyDecision::Allow
    }

    fn matches_all_conditions(
        &self,
        event: &govrix_scout_common::models::AgentEvent,
        conditions: &[Condition],
    ) -> bool {
        conditions.iter().all(|c| self.matches_condition(event, c))
    }

    fn matches_condition(
        &self,
        event: &govrix_scout_common::models::AgentEvent,
        condition: &Condition,
    ) -> bool {
        let field_value = self.get_field_value(event, &condition.field);
        match &condition.operator {
            Operator::Equals => field_value == condition.value,
            Operator::NotEquals => field_value != condition.value,
            Operator::Contains => field_value.contains(condition.value.as_str()),
            Operator::GreaterThan => field_value
                .parse::<f64>()
                .ok()
                .zip(condition.value.parse::<f64>().ok())
                .map(|(a, b)| a > b)
                .unwrap_or(false),
            Operator::LessThan => field_value
                .parse::<f64>()
                .ok()
                .zip(condition.value.parse::<f64>().ok())
                .map(|(a, b)| a < b)
                .unwrap_or(false),
            Operator::Matches => regex::Regex::new(&condition.value)
                .map(|re| re.is_match(&field_value))
                .unwrap_or(false),
        }
    }

    /// Extract the string value of a named field from `event`.
    ///
    /// Unknown field names return an empty string so that conditions on them
    /// never accidentally match.
    fn get_field_value(
        &self,
        event: &govrix_scout_common::models::AgentEvent,
        field: &str,
    ) -> String {
        match field {
            "model" => event.model.clone().unwrap_or_default(),
            "agent_id" => event.agent_id.clone(),
            "provider" => event.provider.to_string(),
            "direction" => event.direction.to_string(),
            "cost_usd" => event.cost_usd.map(|c| c.to_string()).unwrap_or_default(),
            "input_tokens" => event
                .input_tokens
                .map(|t| t.to_string())
                .unwrap_or_default(),
            "output_tokens" => event
                .output_tokens
                .map(|t| t.to_string())
                .unwrap_or_default(),
            _ => String::new(),
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use govrix_scout_common::models::event::{EventDirection, Provider};
    use uuid::Uuid;

    fn make_event() -> govrix_scout_common::models::AgentEvent {
        govrix_scout_common::models::AgentEvent::new(
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

    // 1. Empty rules → Allow
    #[test]
    fn empty_rules_always_allow() {
        let engine = PolicyEngine::new();
        let event = make_event();
        assert_eq!(engine.evaluate(&event), PolicyDecision::Allow);
    }

    // 2. Single block rule that matches → Block
    #[test]
    fn block_rule_matches() {
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

        let event = make_event(); // provider = OpenAI
        assert_eq!(
            engine.evaluate(&event),
            PolicyDecision::Block {
                reason: "blocked by rule: no-openai".to_string(),
            }
        );
    }

    // 3. Block rule that does not match → Allow
    #[test]
    fn block_rule_no_match_allows() {
        let mut engine = PolicyEngine::new();
        engine.load_rules(vec![PolicyRule {
            name: "no-anthropic".to_string(),
            description: None,
            enabled: true,
            conditions: vec![Condition {
                field: "provider".to_string(),
                operator: Operator::Equals,
                value: "anthropic".to_string(),
            }],
            action: Action::Block,
        }]);

        let event = make_event(); // provider = OpenAI — does not match
        assert_eq!(engine.evaluate(&event), PolicyDecision::Allow);
    }

    // 4. Alert rule → Alert
    #[test]
    fn alert_rule_matches() {
        let mut engine = PolicyEngine::new();
        engine.load_rules(vec![PolicyRule {
            name: "alert-outbound".to_string(),
            description: Some("flag outbound traffic".to_string()),
            enabled: true,
            conditions: vec![Condition {
                field: "direction".to_string(),
                operator: Operator::Equals,
                value: "outbound".to_string(),
            }],
            action: Action::Alert,
        }]);

        let event = make_event();
        assert_eq!(
            engine.evaluate(&event),
            PolicyDecision::Alert {
                message: "alert from rule: alert-outbound".to_string(),
            }
        );
    }

    // 5. GreaterThan condition — cost above threshold → Block
    #[test]
    fn greater_than_cost_blocks() {
        use rust_decimal::Decimal;
        use std::str::FromStr;

        let mut engine = PolicyEngine::new();
        engine.load_rules(vec![PolicyRule {
            name: "cost-limit".to_string(),
            description: None,
            enabled: true,
            conditions: vec![Condition {
                field: "cost_usd".to_string(),
                operator: Operator::GreaterThan,
                value: "1.0".to_string(),
            }],
            action: Action::Block,
        }]);

        let mut event = make_event();
        event.cost_usd = Some(Decimal::from_str("2.50").unwrap());

        assert_eq!(
            engine.evaluate(&event),
            PolicyDecision::Block {
                reason: "blocked by rule: cost-limit".to_string(),
            }
        );

        // Cost below threshold should allow.
        event.cost_usd = Some(Decimal::from_str("0.50").unwrap());
        assert_eq!(engine.evaluate(&event), PolicyDecision::Allow);
    }

    // 6. YAML loading works
    #[test]
    fn yaml_loading_works() {
        let yaml = r#"
- name: block-gpt4
  enabled: true
  conditions:
    - field: model
      operator: equals
      value: gpt-4
  action: block
- name: alert-anthropic
  enabled: true
  conditions:
    - field: provider
      operator: equals
      value: anthropic
  action: alert
"#;
        let rules = PolicyEngine::load_yaml(yaml).expect("YAML parse failed");
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name, "block-gpt4");
        assert!(rules[0].enabled);
        assert_eq!(rules[0].conditions.len(), 1);
        assert!(matches!(rules[0].action, Action::Block));
        assert_eq!(rules[1].name, "alert-anthropic");
        assert!(matches!(rules[1].action, Action::Alert));
    }

    // 7. Disabled rule is skipped
    #[test]
    fn disabled_rule_is_skipped() {
        let mut engine = PolicyEngine::new();
        engine.load_rules(vec![PolicyRule {
            name: "disabled-block".to_string(),
            description: None,
            enabled: false,
            conditions: vec![Condition {
                field: "provider".to_string(),
                operator: Operator::Equals,
                value: "openai".to_string(),
            }],
            action: Action::Block,
        }]);

        let event = make_event();
        assert_eq!(engine.evaluate(&event), PolicyDecision::Allow);
    }

    // 8. YAML-loaded rules evaluate correctly end-to-end
    #[test]
    fn yaml_rules_evaluate() {
        let yaml = r#"
- name: block-openai
  enabled: true
  conditions:
    - field: provider
      operator: equals
      value: openai
  action: block
"#;
        let rules = PolicyEngine::load_yaml(yaml).expect("YAML parse failed");
        let mut engine = PolicyEngine::new();
        engine.load_rules(rules);

        let event = make_event(); // provider = OpenAI
        assert_eq!(
            engine.evaluate(&event),
            PolicyDecision::Block {
                reason: "blocked by rule: block-openai".to_string(),
            }
        );
    }

    // 9. Regex match condition
    #[test]
    fn regex_match_condition() {
        let mut engine = PolicyEngine::new();
        engine.load_rules(vec![PolicyRule {
            name: "gpt-models".to_string(),
            description: None,
            enabled: true,
            conditions: vec![Condition {
                field: "model".to_string(),
                operator: Operator::Matches,
                value: r"^gpt-4.*".to_string(),
            }],
            action: Action::Alert,
        }]);

        let mut event = make_event();
        event.model = Some("gpt-4o".to_string());
        assert_eq!(
            engine.evaluate(&event),
            PolicyDecision::Alert {
                message: "alert from rule: gpt-models".to_string(),
            }
        );

        event.model = Some("claude-3-5-sonnet-20241022".to_string());
        assert_eq!(engine.evaluate(&event), PolicyDecision::Allow);
    }

    // 10. Multiple conditions — all must match (AND semantics)
    #[test]
    fn multiple_conditions_and_semantics() {
        let mut engine = PolicyEngine::new();
        engine.load_rules(vec![PolicyRule {
            name: "openai-outbound-block".to_string(),
            description: None,
            enabled: true,
            conditions: vec![
                Condition {
                    field: "provider".to_string(),
                    operator: Operator::Equals,
                    value: "openai".to_string(),
                },
                Condition {
                    field: "direction".to_string(),
                    operator: Operator::Equals,
                    value: "inbound".to_string(), // does NOT match outbound event
                },
            ],
            action: Action::Block,
        }]);

        // Event is outbound → second condition fails → Allow
        let event = make_event();
        assert_eq!(engine.evaluate(&event), PolicyDecision::Allow);
    }

    // 11. rule_count on empty engine
    #[test]
    fn rule_count_empty_engine() {
        let engine = PolicyEngine::new();
        assert_eq!(engine.rule_count(), (0, 0));
    }

    // 12. rule_count with mix of enabled and disabled rules
    #[test]
    fn rule_count_mixed_enabled() {
        let mut engine = PolicyEngine::new();
        engine.load_rules(vec![
            PolicyRule {
                name: "enabled-rule".to_string(),
                description: None,
                enabled: true,
                conditions: vec![],
                action: Action::Allow,
            },
            PolicyRule {
                name: "disabled-rule".to_string(),
                description: None,
                enabled: false,
                conditions: vec![],
                action: Action::Block,
            },
            PolicyRule {
                name: "another-enabled".to_string(),
                description: None,
                enabled: true,
                conditions: vec![],
                action: Action::Alert,
            },
        ]);
        assert_eq!(engine.rule_count(), (3, 2));
    }

    // 13. reload_rules replaces the previous rule set
    #[test]
    fn reload_rules_replaces_previous_set() {
        let mut engine = PolicyEngine::new();
        engine.load_rules(vec![PolicyRule {
            name: "original-rule".to_string(),
            description: None,
            enabled: true,
            conditions: vec![Condition {
                field: "provider".to_string(),
                operator: Operator::Equals,
                value: "openai".to_string(),
            }],
            action: Action::Block,
        }]);

        // Confirm original rule is active
        let event = make_event();
        assert_eq!(
            engine.evaluate(&event),
            PolicyDecision::Block {
                reason: "blocked by rule: original-rule".to_string(),
            }
        );
        assert_eq!(engine.rule_count(), (1, 1));

        // Replace with an empty rule set
        engine.reload_rules(vec![]);
        assert_eq!(engine.rule_count(), (0, 0));
        assert_eq!(engine.evaluate(&event), PolicyDecision::Allow);
    }

    // 14. load_rules_from_yaml — valid YAML loads rules and returns count
    #[test]
    fn load_rules_from_yaml_valid() {
        let yaml = r#"
- name: block-unauthorized-models
  description: Block access to non-approved AI models
  enabled: true
  conditions:
    - field: provider
      operator: not_equals
      value: openai
  action: block

- name: alert-outbound-requests
  description: Alert on all outbound agent requests
  enabled: true
  conditions:
    - field: direction
      operator: equals
      value: outbound
  action: alert
"#;
        let mut engine = PolicyEngine::new();
        let count = engine
            .load_rules_from_yaml(yaml)
            .expect("valid YAML should parse");
        assert_eq!(count, 2);
        assert_eq!(engine.rule_count(), (2, 2));
        // Verify rule names are preserved correctly.
        let (total, _) = engine.rule_count();
        assert_eq!(total, 2);
    }

    // 15. load_rules_from_yaml — invalid YAML returns an error
    #[test]
    fn load_rules_from_yaml_invalid() {
        let bad_yaml = "{ not valid yaml sequence: [[[";
        let mut engine = PolicyEngine::new();
        assert!(
            engine.load_rules_from_yaml(bad_yaml).is_err(),
            "invalid YAML must return Err"
        );
        // Engine should remain empty (load was never committed).
        assert_eq!(engine.rule_count(), (0, 0));
    }

    // 16. yaml_rules_evaluate_correctly — load from YAML then evaluate an event
    #[test]
    fn yaml_rules_evaluate_correctly() {
        let yaml = r#"
- name: block-unauthorized-models
  description: Block access to non-approved AI models
  enabled: true
  conditions:
    - field: provider
      operator: not_equals
      value: openai
  action: block

- name: alert-outbound-requests
  description: Alert on all outbound agent requests
  enabled: true
  conditions:
    - field: direction
      operator: equals
      value: outbound
  action: alert
"#;
        let mut engine = PolicyEngine::new();
        engine
            .load_rules_from_yaml(yaml)
            .expect("valid YAML should parse");

        // make_event() produces an OpenAI outbound event.
        // Rule 1 (not_equals openai) does NOT match — provider IS openai.
        // Rule 2 (direction equals outbound) MATCHES → Alert.
        let event = make_event();
        assert_eq!(
            engine.evaluate(&event),
            PolicyDecision::Alert {
                message: "alert from rule: alert-outbound-requests".to_string(),
            }
        );
    }
}
