//! Cost breakdown — compute USD cost estimates from event data.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::usage::EventSummary;

/// Per-agent cost roll-up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCost {
    /// The agent identifier.
    pub agent_id: String,
    /// Estimated cost in USD for this agent.
    pub cost_usd: f64,
    /// Total tokens consumed.
    pub tokens: u64,
    /// Total requests attributed to this agent.
    pub requests: u64,
}

/// Workspace-level cost breakdown for a set of events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// Total estimated cost in USD across all agents.
    pub total_cost_usd: f64,
    /// Total tokens consumed.
    pub total_tokens: u64,
    /// Per-agent cost breakdown, sorted by cost descending.
    pub by_agent: Vec<AgentCost>,
}

/// Default cost per 1 000 tokens in USD used when no per-event cost is
/// available. This is a conservative estimate; real costs depend on the model.
const COST_PER_1K_TOKENS: f64 = 0.002;

impl CostBreakdown {
    /// Aggregate [`EventSummary`] records into a [`CostBreakdown`].
    ///
    /// Each event's token count is converted to an estimated USD cost using the
    /// workspace default rate [`COST_PER_1K_TOKENS`].
    pub fn from_events(events: &[EventSummary]) -> Self {
        let mut by_agent: HashMap<String, (u64, u64)> = HashMap::new();

        for event in events {
            let entry = by_agent.entry(event.agent_id.clone()).or_default();
            entry.0 += event.tokens;
            entry.1 += event.requests;
        }

        let total_tokens: u64 = by_agent.values().map(|(t, _)| t).sum();

        let mut agents: Vec<AgentCost> = by_agent
            .into_iter()
            .map(|(agent_id, (tokens, requests))| {
                let cost_usd = (tokens as f64 / 1000.0) * COST_PER_1K_TOKENS;
                AgentCost {
                    agent_id,
                    cost_usd,
                    tokens,
                    requests,
                }
            })
            .collect();

        // Deterministic ordering: highest cost first
        agents.sort_by(|a, b| {
            b.cost_usd
                .partial_cmp(&a.cost_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_cost_usd = agents.iter().map(|a| a.cost_usd).sum();

        Self {
            total_cost_usd,
            total_tokens,
            by_agent: agents,
        }
    }

    /// Render this breakdown as a Markdown table.
    pub fn to_markdown(&self) -> String {
        let mut out = String::from("# Cost Breakdown\n\n");
        out.push_str("| Metric | Value |\n");
        out.push_str("|--------|-------|\n");
        out.push_str(&format!(
            "| Total Cost (USD) | ${:.6} |\n",
            self.total_cost_usd
        ));
        out.push_str(&format!("| Total Tokens | {} |\n", self.total_tokens));

        if !self.by_agent.is_empty() {
            out.push_str("\n## By Agent\n\n");
            out.push_str("| Agent | Cost (USD) | Tokens | Requests |\n");
            out.push_str("|-------|-----------|--------|----------|\n");
            for a in &self.by_agent {
                out.push_str(&format!(
                    "| {} | ${:.6} | {} | {} |\n",
                    a.agent_id, a.cost_usd, a.tokens, a.requests
                ));
            }
        }

        out
    }

    /// Serialise this breakdown to a pretty-printed JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("CostBreakdown is always serialisable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_breakdown_aggregates() {
        let events = vec![
            EventSummary {
                agent_id: "a1".into(),
                tokens: 1000,
                requests: 1,
            },
            EventSummary {
                agent_id: "a2".into(),
                tokens: 2000,
                requests: 2,
            },
        ];
        let breakdown = CostBreakdown::from_events(&events);
        assert_eq!(breakdown.total_tokens, 3000);
        // 3 * (0.002 / 1000) * 1000 = 3 * 0.002 = 0.006
        let expected = (3000.0f64 / 1000.0) * COST_PER_1K_TOKENS;
        let diff = (breakdown.total_cost_usd - expected).abs();
        assert!(diff < 1e-9, "cost mismatch: {}", diff);
    }

    #[test]
    fn cost_renders_to_markdown() {
        let breakdown = CostBreakdown {
            total_cost_usd: 0.042,
            total_tokens: 21000,
            by_agent: vec![],
        };
        let md = breakdown.to_markdown();
        assert!(md.contains("# Cost Breakdown"));
        assert!(md.contains("Total Cost"));
    }
}
