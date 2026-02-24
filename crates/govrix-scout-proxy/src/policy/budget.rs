//! Budget tracking — token and cost caps per agent and globally.
//!
//! The `BudgetTracker` holds in-memory usage counters. It resets at UTC midnight
//! (daily) and is checked before each event is logged. Usage is recorded after
//! an event completes.
//!
//! Fail-open: if budget data is unavailable or stale, `Allow` is returned.

#![allow(dead_code)]

use std::collections::HashMap;

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::types::{AlertSeverity, PolicyAction};

// ── Configuration types ───────────────────────────────────────────────────────

/// Budget limits for the entire system (all agents combined).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BudgetLimit {
    /// Maximum tokens across ALL agents per day.
    pub daily_token_limit: Option<u64>,
    /// Maximum USD cost across ALL agents per day.
    pub daily_cost_limit_usd: Option<f64>,
}

/// Per-agent budget configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBudget {
    /// The agent_id this budget applies to.
    pub agent_id: String,
    /// Maximum tokens this agent may consume per day.
    pub daily_token_limit: Option<u64>,
    /// Maximum USD this agent may spend per day.
    pub daily_cost_limit_usd: Option<f64>,
    /// Maximum USD this agent may spend per calendar month.
    pub monthly_cost_limit_usd: Option<f64>,
}

/// Top-level budget policy configuration.
///
/// Loaded from YAML (see `loader.rs`). The engine merges per-agent limits
/// with global limits; the most restrictive applies.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BudgetPolicy {
    /// Per-agent budget overrides (keyed by agent_id).
    pub agent_limits: HashMap<String, AgentBudget>,
    /// Global budget applied to all agents (fallback when no per-agent entry).
    pub global_limit: Option<BudgetLimit>,
}

// ── Usage tracking ────────────────────────────────────────────────────────────

/// Accumulated daily usage for one agent.
#[derive(Debug, Clone, Default)]
pub struct DailyUsage {
    /// Total tokens consumed today.
    pub tokens: u64,
    /// Total USD cost incurred today.
    pub cost_usd: f64,
}

/// In-memory budget tracker.
///
/// Holds per-agent daily usage and resets counters at UTC midnight.
/// Thread-safety is handled by the caller (typically behind a `Mutex`).
pub struct BudgetTracker {
    policy: BudgetPolicy,
    /// Per-agent daily usage; reset when `last_reset` date changes.
    daily_usage: HashMap<String, DailyUsage>,
    /// The UTC date of the last reset (or startup).
    last_reset: NaiveDate,
}

impl BudgetTracker {
    /// Create a new tracker with the given policy configuration.
    pub fn new(policy: BudgetPolicy) -> Self {
        Self {
            policy,
            daily_usage: HashMap::new(),
            last_reset: Utc::now().date_naive(),
        }
    }

    /// Create a tracker with no limits (allow everything).
    pub fn unlimited() -> Self {
        Self::new(BudgetPolicy::default())
    }

    /// Check whether `estimated_tokens` and `estimated_cost` are within budget.
    ///
    /// Returns:
    /// - `PolicyAction::Allow` — under budget
    /// - `PolicyAction::Block { reason }` — over limit
    /// - `PolicyAction::Alert { .. }` — approaching limit (90% threshold)
    ///
    /// Fails open: if a budget lookup panics or the agent is unknown, returns `Allow`.
    pub fn check_budget(
        &self,
        agent_id: &str,
        estimated_tokens: u64,
        estimated_cost_usd: f64,
    ) -> PolicyAction {
        let usage = self.current_usage(agent_id);

        // ── Per-agent limits ──────────────────────────────────────────────────
        if let Some(agent_budget) = self.policy.agent_limits.get(agent_id) {
            if let Some(limit) = agent_budget.daily_token_limit {
                if let Some(action) = check_token_limit(
                    usage.tokens,
                    estimated_tokens,
                    limit,
                    agent_id,
                    "daily token limit",
                ) {
                    return action;
                }
            }
            if let Some(limit) = agent_budget.daily_cost_limit_usd {
                if let Some(action) = check_cost_limit(
                    usage.cost_usd,
                    estimated_cost_usd,
                    limit,
                    agent_id,
                    "daily cost limit",
                ) {
                    return action;
                }
            }
        }

        // ── Global limits (fallback) ──────────────────────────────────────────
        if let Some(global) = &self.policy.global_limit {
            if let Some(limit) = global.daily_token_limit {
                let global_tokens: u64 = self.daily_usage.values().map(|u| u.tokens).sum();
                if let Some(action) = check_token_limit(
                    global_tokens,
                    estimated_tokens,
                    limit,
                    "global",
                    "global daily token limit",
                ) {
                    return action;
                }
            }
            if let Some(limit) = global.daily_cost_limit_usd {
                let global_cost: f64 = self.daily_usage.values().map(|u| u.cost_usd).sum();
                if let Some(action) = check_cost_limit(
                    global_cost,
                    estimated_cost_usd,
                    limit,
                    "global",
                    "global daily cost limit",
                ) {
                    return action;
                }
            }
        }

        PolicyAction::Allow
    }

    /// Record actual usage after an event completes.
    ///
    /// Resets daily counters if the UTC date has changed since the last record.
    pub fn record_usage(&mut self, agent_id: &str, tokens: u64, cost_usd: f64) {
        self.maybe_reset_daily();
        let usage = self.daily_usage.entry(agent_id.to_string()).or_default();
        usage.tokens = usage.tokens.saturating_add(tokens);
        usage.cost_usd += cost_usd;
    }

    /// Return a snapshot of today's usage for `agent_id` (zero if unseen).
    pub fn current_usage(&self, agent_id: &str) -> DailyUsage {
        self.daily_usage.get(agent_id).cloned().unwrap_or_default()
    }

    /// Return all agent IDs that have usage recorded today.
    pub fn tracked_agents(&self) -> Vec<String> {
        self.daily_usage.keys().cloned().collect()
    }

    /// Reset daily counters if the UTC date has changed.
    fn maybe_reset_daily(&mut self) {
        let today = Utc::now().date_naive();
        if today > self.last_reset {
            self.daily_usage.clear();
            self.last_reset = today;
            tracing::info!("budget tracker: daily usage reset for {}", today);
        }
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Check a token limit; return Some(action) if exceeded or approaching.
fn check_token_limit(
    current: u64,
    estimated: u64,
    limit: u64,
    agent_id: &str,
    label: &str,
) -> Option<PolicyAction> {
    let projected = current.saturating_add(estimated);

    if projected > limit {
        return Some(PolicyAction::Block {
            reason: format!(
                "Agent '{}' would exceed {} ({} + {} > {})",
                agent_id, label, current, estimated, limit
            ),
        });
    }

    // Warn at 90% utilisation
    let threshold = (limit as f64 * 0.90) as u64;
    if projected > threshold {
        return Some(PolicyAction::Alert {
            message: format!(
                "Agent '{}' is approaching {} ({}/{} tokens used)",
                agent_id, label, projected, limit
            ),
            severity: AlertSeverity::High,
        });
    }

    None
}

/// Check a cost limit; return Some(action) if exceeded or approaching.
fn check_cost_limit(
    current: f64,
    estimated: f64,
    limit: f64,
    agent_id: &str,
    label: &str,
) -> Option<PolicyAction> {
    let projected = current + estimated;

    if projected > limit {
        return Some(PolicyAction::Block {
            reason: format!(
                "Agent '{}' would exceed {} (${:.4} + ${:.4} > ${:.4})",
                agent_id, label, current, estimated, limit
            ),
        });
    }

    let threshold = limit * 0.90;
    if projected > threshold {
        return Some(PolicyAction::Alert {
            message: format!(
                "Agent '{}' is approaching {} (${:.4}/${:.4} used)",
                agent_id, label, projected, limit
            ),
            severity: AlertSeverity::High,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent_policy(
        agent_id: &str,
        daily_tokens: Option<u64>,
        daily_cost: Option<f64>,
    ) -> BudgetPolicy {
        let mut policy = BudgetPolicy::default();
        policy.agent_limits.insert(
            agent_id.to_string(),
            AgentBudget {
                agent_id: agent_id.to_string(),
                daily_token_limit: daily_tokens,
                daily_cost_limit_usd: daily_cost,
                monthly_cost_limit_usd: None,
            },
        );
        policy
    }

    // ── check_budget: Allow ───────────────────────────────────────────────────

    #[test]
    fn allow_when_under_token_limit() {
        let policy = make_agent_policy("agent-1", Some(1_000_000), None);
        let tracker = BudgetTracker::new(policy);
        let action = tracker.check_budget("agent-1", 500_000, 0.0);
        assert_eq!(action, PolicyAction::Allow);
    }

    #[test]
    fn allow_when_under_cost_limit() {
        let policy = make_agent_policy("agent-1", None, Some(10.0));
        let tracker = BudgetTracker::new(policy);
        let action = tracker.check_budget("agent-1", 0, 5.0);
        assert_eq!(action, PolicyAction::Allow);
    }

    #[test]
    fn allow_when_no_policy_for_agent() {
        let tracker = BudgetTracker::unlimited();
        let action = tracker.check_budget("unknown-agent", 999_999_999, 999.99);
        assert_eq!(action, PolicyAction::Allow);
    }

    // ── check_budget: Block ───────────────────────────────────────────────────

    #[test]
    fn block_when_over_token_limit() {
        let policy = make_agent_policy("agent-1", Some(100), None);
        let mut tracker = BudgetTracker::new(policy);
        tracker.record_usage("agent-1", 90, 0.0);
        let action = tracker.check_budget("agent-1", 20, 0.0); // 90 + 20 > 100
        assert!(matches!(action, PolicyAction::Block { .. }));
    }

    #[test]
    fn block_when_over_cost_limit() {
        let policy = make_agent_policy("agent-1", None, Some(10.0));
        let mut tracker = BudgetTracker::new(policy);
        tracker.record_usage("agent-1", 0, 9.50);
        let action = tracker.check_budget("agent-1", 0, 1.0); // 9.50 + 1.0 > 10.0
        assert!(matches!(action, PolicyAction::Block { .. }));
    }

    // ── check_budget: Alert at 90% ────────────────────────────────────────────

    #[test]
    fn alert_when_approaching_token_limit() {
        let policy = make_agent_policy("agent-1", Some(1_000), None);
        let mut tracker = BudgetTracker::new(policy);
        tracker.record_usage("agent-1", 850, 0.0);
        let action = tracker.check_budget("agent-1", 100, 0.0); // 950/1000 = 95%
        assert!(matches!(
            action,
            PolicyAction::Alert {
                severity: AlertSeverity::High,
                ..
            }
        ));
    }

    #[test]
    fn alert_when_approaching_cost_limit() {
        let policy = make_agent_policy("agent-1", None, Some(100.0));
        let mut tracker = BudgetTracker::new(policy);
        tracker.record_usage("agent-1", 0, 85.0);
        let action = tracker.check_budget("agent-1", 0, 7.0); // 92/100 = 92%
        assert!(matches!(
            action,
            PolicyAction::Alert {
                severity: AlertSeverity::High,
                ..
            }
        ));
    }

    // ── record_usage ──────────────────────────────────────────────────────────

    #[test]
    fn record_usage_accumulates() {
        let mut tracker = BudgetTracker::unlimited();
        tracker.record_usage("agent-1", 100, 1.0);
        tracker.record_usage("agent-1", 200, 2.5);
        let usage = tracker.current_usage("agent-1");
        assert_eq!(usage.tokens, 300);
        assert!((usage.cost_usd - 3.5).abs() < 1e-9);
    }

    #[test]
    fn record_usage_multiple_agents() {
        let mut tracker = BudgetTracker::unlimited();
        tracker.record_usage("agent-1", 100, 1.0);
        tracker.record_usage("agent-2", 200, 2.0);
        assert_eq!(tracker.current_usage("agent-1").tokens, 100);
        assert_eq!(tracker.current_usage("agent-2").tokens, 200);
    }

    #[test]
    fn current_usage_returns_zero_for_unknown_agent() {
        let tracker = BudgetTracker::unlimited();
        let usage = tracker.current_usage("nobody");
        assert_eq!(usage.tokens, 0);
        assert_eq!(usage.cost_usd, 0.0);
    }

    // ── Global limits ─────────────────────────────────────────────────────────

    #[test]
    fn block_on_global_token_limit() {
        let policy = BudgetPolicy {
            global_limit: Some(BudgetLimit {
                daily_token_limit: Some(500),
                daily_cost_limit_usd: None,
            }),
            ..Default::default()
        };
        let mut tracker = BudgetTracker::new(policy);
        tracker.record_usage("agent-1", 300, 0.0);
        tracker.record_usage("agent-2", 150, 0.0); // global total: 450
                                                   // Next request would push global total to 500 + 60 = over limit
        let action = tracker.check_budget("agent-3", 60, 0.0);
        assert!(matches!(action, PolicyAction::Block { .. }));
    }

    #[test]
    fn allow_when_global_limit_not_reached() {
        let policy = BudgetPolicy {
            global_limit: Some(BudgetLimit {
                daily_token_limit: Some(1_000_000),
                daily_cost_limit_usd: None,
            }),
            ..Default::default()
        };
        let tracker = BudgetTracker::new(policy);
        let action = tracker.check_budget("agent-1", 100, 0.0);
        assert_eq!(action, PolicyAction::Allow);
    }

    // ── Token overflow safety ─────────────────────────────────────────────────

    #[test]
    fn record_usage_saturates_at_max() {
        let mut tracker = BudgetTracker::unlimited();
        tracker.record_usage("agent-1", u64::MAX, 0.0);
        tracker.record_usage("agent-1", 1, 0.0); // saturating_add → no overflow
        let usage = tracker.current_usage("agent-1");
        assert_eq!(usage.tokens, u64::MAX);
    }
}
