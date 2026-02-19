//! YAML policy loader — deserialize policy configuration from disk.
//!
//! Policies are loaded from a YAML file at startup (typically `config/policies.yaml`).
//! The `PolicyConfig` struct maps 1:1 to the YAML schema documented below.
//!
//! Example YAML:
//! ```yaml
//! policies:
//!   - id: pii-detection
//!     name: "PII Detection"
//!     enabled: true
//!     rules:
//!       - scan_for: [email, phone, ssn, credit_card]
//!         action: alert
//!         severity: high
//!   - id: budget-cap
//!     name: "Daily Budget Cap"
//!     enabled: true
//!     rules:
//!       - agent_class: "*"
//!         daily_cost_limit_usd: 100.0
//!         daily_token_limit: 1000000
//!         action: block
//! ```

#![allow(dead_code)]

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::budget::{AgentBudget, BudgetLimit, BudgetPolicy};

// ── Raw YAML schema ───────────────────────────────────────────────────────────

/// Top-level configuration file structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolicyConfig {
    /// List of policy entries from the YAML file.
    #[serde(default)]
    pub policies: Vec<PolicyEntry>,
}

/// One policy block in the YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEntry {
    /// Unique ID for this policy (used as `rule_id` in decisions).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// If `false`, all rules in this policy are skipped.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// The actual rules within this policy.
    #[serde(default)]
    pub rules: Vec<PolicyRuleEntry>,
}

/// One rule within a policy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolicyRuleEntry {
    // ── PII detection rule fields ──────────────────────────────────────────
    /// PII types to scan for. When present, the rule fires as a PII scan rule.
    /// Valid values: `email`, `phone`, `ssn`, `credit_card`, `ip_address`
    #[serde(default)]
    pub scan_for: Vec<String>,

    // ── Budget rule fields ────────────────────────────────────────────────
    /// Agent class: `"*"` (all) or a specific agent_id.
    pub agent_class: Option<String>,
    /// Daily token limit for this class.
    pub daily_token_limit: Option<u64>,
    /// Daily USD cost limit.
    pub daily_cost_limit_usd: Option<f64>,
    /// Monthly USD cost limit (per-agent only, not global).
    pub monthly_cost_limit_usd: Option<f64>,

    // ── Action fields ─────────────────────────────────────────────────────
    /// Action to take: `"allow"`, `"block"`, `"alert"`, `"redact"`.
    #[serde(default)]
    pub action: String,
    /// Severity for `"alert"` actions: `"low"`, `"medium"`, `"high"`, `"critical"`.
    pub severity: Option<String>,
}

fn default_true() -> bool {
    true
}

// ── Loading ───────────────────────────────────────────────────────────────────

/// Load `PolicyConfig` from a YAML file at `path`.
///
/// Returns `Ok(PolicyConfig::default())` if the file does not exist (no policies).
/// Returns `Err` if the file exists but cannot be parsed.
pub fn load_from_file(path: &Path) -> anyhow::Result<PolicyConfig> {
    if !path.exists() {
        tracing::info!(
            path = %path.display(),
            "policy file not found — using defaults (no policies)"
        );
        return Ok(PolicyConfig::default());
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read policy file {}: {}", path.display(), e))?;

    let config: PolicyConfig = serde_yaml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse policy file {}: {}", path.display(), e))?;

    tracing::info!(
        path = %path.display(),
        policy_count = config.policies.len(),
        "loaded policy configuration"
    );

    Ok(config)
}

/// Load `PolicyConfig` from a YAML string (for tests and embedded configs).
pub fn load_from_str(yaml: &str) -> anyhow::Result<PolicyConfig> {
    let config: PolicyConfig = serde_yaml::from_str(yaml)
        .map_err(|e| anyhow::anyhow!("failed to parse policy YAML: {}", e))?;
    Ok(config)
}

// ── Conversion: PolicyConfig → BudgetPolicy ───────────────────────────────────

/// Extract a `BudgetPolicy` from the loaded `PolicyConfig`.
///
/// Scans all enabled policies for budget-cap rules and builds the
/// `BudgetPolicy` struct used by `BudgetTracker`.
pub fn extract_budget_policy(config: &PolicyConfig) -> BudgetPolicy {
    let mut budget = BudgetPolicy::default();

    for entry in &config.policies {
        if !entry.enabled {
            continue;
        }

        // Look for budget-cap policies by ID prefix or budget-relevant fields
        let is_budget_policy = entry.id.contains("budget")
            || entry.rules.iter().any(|r| {
                r.daily_token_limit.is_some()
                    || r.daily_cost_limit_usd.is_some()
                    || r.monthly_cost_limit_usd.is_some()
            });

        if !is_budget_policy {
            continue;
        }

        for rule in &entry.rules {
            let agent_class = rule.agent_class.as_deref().unwrap_or("*");

            if agent_class == "*" {
                // Global limit
                let global = budget.global_limit.get_or_insert(BudgetLimit::default());
                if let Some(t) = rule.daily_token_limit {
                    global.daily_token_limit = Some(t);
                }
                if let Some(c) = rule.daily_cost_limit_usd {
                    global.daily_cost_limit_usd = Some(c);
                }
            } else {
                // Per-agent limit
                let agent_budget = budget
                    .agent_limits
                    .entry(agent_class.to_string())
                    .or_insert_with(|| AgentBudget {
                        agent_id: agent_class.to_string(),
                        daily_token_limit: None,
                        daily_cost_limit_usd: None,
                        monthly_cost_limit_usd: None,
                    });
                if let Some(t) = rule.daily_token_limit {
                    agent_budget.daily_token_limit = Some(t);
                }
                if let Some(c) = rule.daily_cost_limit_usd {
                    agent_budget.daily_cost_limit_usd = Some(c);
                }
                if let Some(m) = rule.monthly_cost_limit_usd {
                    agent_budget.monthly_cost_limit_usd = Some(m);
                }
            }
        }
    }

    budget
}

/// Extract the list of PII types to scan for from the `PolicyConfig`.
///
/// Returns the union of all `scan_for` arrays from enabled PII policies.
/// Result is deduplicated.
pub fn extract_pii_scan_types(config: &PolicyConfig) -> Vec<String> {
    let mut types = Vec::new();

    for entry in &config.policies {
        if !entry.enabled {
            continue;
        }
        for rule in &entry.rules {
            for scan_type in &rule.scan_for {
                if !types.contains(scan_type) {
                    types.push(scan_type.clone());
                }
            }
        }
    }

    types
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_YAML: &str = r#"
policies:
  - id: pii-detection
    name: "PII Detection"
    enabled: true
    rules:
      - scan_for: [email, phone, ssn, credit_card]
        action: alert
        severity: high
  - id: budget-cap
    name: "Daily Budget Cap"
    enabled: true
    rules:
      - agent_class: "*"
        daily_cost_limit_usd: 100.0
        daily_token_limit: 1000000
        action: block
"#;

    #[test]
    fn load_example_yaml() {
        let config = load_from_str(EXAMPLE_YAML).unwrap();
        assert_eq!(config.policies.len(), 2);
        assert_eq!(config.policies[0].id, "pii-detection");
        assert_eq!(config.policies[1].id, "budget-cap");
    }

    #[test]
    fn pii_policy_enabled_by_default() {
        let config = load_from_str(EXAMPLE_YAML).unwrap();
        assert!(config.policies[0].enabled);
    }

    #[test]
    fn disabled_policy_parsed() {
        let yaml = r#"
policies:
  - id: disabled-test
    name: "Disabled"
    enabled: false
    rules: []
"#;
        let config = load_from_str(yaml).unwrap();
        assert!(!config.policies[0].enabled);
    }

    #[test]
    fn extract_budget_policy_global_limit() {
        let config = load_from_str(EXAMPLE_YAML).unwrap();
        let budget = extract_budget_policy(&config);
        let global = budget.global_limit.expect("global limit present");
        assert_eq!(global.daily_cost_limit_usd, Some(100.0));
        assert_eq!(global.daily_token_limit, Some(1_000_000));
    }

    #[test]
    fn extract_budget_policy_per_agent() {
        let yaml = r#"
policies:
  - id: budget-cap
    name: "Agent Budget"
    enabled: true
    rules:
      - agent_class: "agent-007"
        daily_cost_limit_usd: 50.0
        daily_token_limit: 500000
        action: block
"#;
        let config = load_from_str(yaml).unwrap();
        let budget = extract_budget_policy(&config);
        let agent = budget
            .agent_limits
            .get("agent-007")
            .expect("agent-007 present");
        assert_eq!(agent.daily_cost_limit_usd, Some(50.0));
        assert_eq!(agent.daily_token_limit, Some(500_000));
    }

    #[test]
    fn extract_pii_types_deduped() {
        let yaml = r#"
policies:
  - id: pii-1
    name: "PII 1"
    enabled: true
    rules:
      - scan_for: [email, phone]
        action: alert
        severity: high
  - id: pii-2
    name: "PII 2"
    enabled: true
    rules:
      - scan_for: [email, ssn]
        action: alert
        severity: high
"#;
        let config = load_from_str(yaml).unwrap();
        let types = extract_pii_scan_types(&config);
        // Should have email, phone, ssn (email deduped)
        assert_eq!(types.len(), 3);
        assert!(types.contains(&"email".to_string()));
        assert!(types.contains(&"phone".to_string()));
        assert!(types.contains(&"ssn".to_string()));
    }

    #[test]
    fn disabled_policy_not_included_in_budget() {
        let yaml = r#"
policies:
  - id: budget-cap
    name: "Budget Cap"
    enabled: false
    rules:
      - agent_class: "*"
        daily_cost_limit_usd: 10.0
        action: block
"#;
        let config = load_from_str(yaml).unwrap();
        let budget = extract_budget_policy(&config);
        assert!(budget.global_limit.is_none());
    }

    #[test]
    fn empty_yaml_produces_default_config() {
        let config = load_from_str("policies: []").unwrap();
        assert!(config.policies.is_empty());
    }

    #[test]
    fn invalid_yaml_returns_error() {
        let result = load_from_str("{{invalid: yaml: [}");
        assert!(result.is_err());
    }

    #[test]
    fn load_from_nonexistent_file_returns_default() {
        let path = Path::new("/tmp/agentmesh_nonexistent_policy_xyz.yaml");
        let config = load_from_file(path).unwrap();
        assert!(config.policies.is_empty());
    }
}
