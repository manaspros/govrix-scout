//! Usage summary — aggregate event data into per-agent usage statistics.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A single event record for usage aggregation purposes.
///
/// This is a simplified projection of a proxy event, containing only the
/// fields required for usage aggregation. It can be constructed from the
/// REST API response or directly in tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    /// The agent that generated the event.
    pub agent_id: String,
    /// Total tokens (input + output) for this event.
    pub tokens: u64,
    /// Number of requests (usually 1 per event).
    pub requests: u64,
}

/// Per-agent usage roll-up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsage {
    /// The agent identifier.
    pub agent_id: String,
    /// Total token usage for this agent across all events.
    pub tokens: u64,
    /// Total request count for this agent.
    pub requests: u64,
}

/// Workspace-level usage summary for a set of events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    /// Total number of proxy requests across all agents.
    pub total_requests: u64,
    /// Total tokens consumed across all agents.
    pub total_tokens: u64,
    /// Number of distinct agents that appear in the events.
    pub unique_agents: usize,
    /// Per-agent breakdown, sorted by token usage descending.
    pub by_agent: Vec<AgentUsage>,
}

impl UsageSummary {
    /// Aggregate a slice of [`EventSummary`] records into a [`UsageSummary`].
    pub fn from_events(events: &[EventSummary]) -> Self {
        let mut by_agent: HashMap<String, (u64, u64)> = HashMap::new();

        for event in events {
            let entry = by_agent.entry(event.agent_id.clone()).or_default();
            entry.0 += event.tokens;
            entry.1 += event.requests;
        }

        let total_tokens = by_agent.values().map(|(t, _)| t).sum();
        let total_requests = by_agent.values().map(|(_, r)| r).sum();
        let unique_agents = by_agent.len();

        let mut agents: Vec<AgentUsage> = by_agent
            .into_iter()
            .map(|(agent_id, (tokens, requests))| AgentUsage {
                agent_id,
                tokens,
                requests,
            })
            .collect();

        // Deterministic ordering: highest token usage first
        agents.sort_by(|a, b| b.tokens.cmp(&a.tokens));

        Self {
            total_requests,
            total_tokens,
            unique_agents,
            by_agent: agents,
        }
    }

    /// Render this summary as a Markdown table.
    pub fn to_markdown(&self) -> String {
        let mut out = String::from("# Usage Summary\n\n");
        out.push_str("| Metric | Value |\n");
        out.push_str("|--------|-------|\n");
        out.push_str(&format!("| Requests | {} |\n", self.total_requests));
        out.push_str(&format!("| Tokens | {} |\n", self.total_tokens));
        out.push_str(&format!("| Unique Agents | {} |\n", self.unique_agents));

        if !self.by_agent.is_empty() {
            out.push_str("\n## By Agent\n\n");
            out.push_str("| Agent | Tokens | Requests |\n");
            out.push_str("|-------|--------|----------|\n");
            for a in &self.by_agent {
                out.push_str(&format!(
                    "| {} | {} | {} |\n",
                    a.agent_id, a.tokens, a.requests
                ));
            }
        }

        out
    }

    /// Serialise this summary to a pretty-printed JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("UsageSummary is always serialisable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_summary_aggregates_correctly() {
        let events = vec![
            EventSummary {
                agent_id: "a1".into(),
                tokens: 100,
                requests: 1,
            },
            EventSummary {
                agent_id: "a1".into(),
                tokens: 200,
                requests: 1,
            },
            EventSummary {
                agent_id: "a2".into(),
                tokens: 50,
                requests: 1,
            },
        ];
        let report = UsageSummary::from_events(&events);
        assert_eq!(report.total_requests, 3);
        assert_eq!(report.total_tokens, 350);
        assert_eq!(report.unique_agents, 2);
    }

    #[test]
    fn usage_renders_to_markdown() {
        let report = UsageSummary {
            total_requests: 10,
            total_tokens: 1000,
            unique_agents: 2,
            by_agent: vec![],
        };
        let md = report.to_markdown();
        assert!(md.contains("# Usage Summary"));
        assert!(md.contains("10"));
    }
}
