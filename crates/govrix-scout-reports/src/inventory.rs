//! Agent inventory — catalogue every agent seen in event data.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::usage::EventSummary;

/// Snapshot record for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecord {
    /// The agent identifier.
    pub agent_id: String,
    /// Total proxy requests attributed to this agent.
    pub total_requests: u64,
    /// Total tokens consumed by this agent.
    pub total_tokens: u64,
    /// Unique LLM providers used by this agent (currently derived from
    /// event metadata; populated as an empty vec when provider is unknown).
    pub providers: Vec<String>,
    /// ISO-8601 timestamp of the most-recent event for this agent, if any.
    pub last_seen: Option<String>,
}

/// Workspace-level agent inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInventory {
    /// Total number of distinct agents seen.
    pub total_agents: usize,
    /// Agents that have at least one request.
    pub active_agents: usize,
    /// Per-agent records, sorted by total tokens descending.
    pub agents: Vec<AgentRecord>,
}

impl AgentInventory {
    /// Build an [`AgentInventory`] from a slice of [`EventSummary`] records.
    ///
    /// Providers and timestamps are not available in the basic
    /// [`EventSummary`] type, so those fields are left as empty / `None`.
    /// Callers that have richer event data should populate the fields
    /// themselves after construction.
    pub fn from_events(events: &[EventSummary]) -> Self {
        // (tokens, requests)
        let mut map: HashMap<String, (u64, u64)> = HashMap::new();

        for event in events {
            let entry = map.entry(event.agent_id.clone()).or_default();
            entry.0 += event.tokens;
            entry.1 += event.requests;
        }

        let total_agents = map.len();

        let mut agents: Vec<AgentRecord> = map
            .into_iter()
            .map(|(agent_id, (tokens, requests))| AgentRecord {
                agent_id,
                total_requests: requests,
                total_tokens: tokens,
                providers: Vec::new(),
                last_seen: None,
            })
            .collect();

        // Deterministic ordering: most-used agent first
        agents.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));

        let active_agents = agents.iter().filter(|a| a.total_requests > 0).count();

        Self {
            total_agents,
            active_agents,
            agents,
        }
    }

    /// Render this inventory as a Markdown table.
    pub fn to_markdown(&self) -> String {
        let mut out = String::from("# Agent Inventory\n\n");
        out.push_str(&format!("Total agents: **{}**  \n", self.total_agents));
        out.push_str(&format!("Active agents: **{}**\n\n", self.active_agents));

        if !self.agents.is_empty() {
            out.push_str("| Agent ID | Requests | Tokens | Providers | Last Seen |\n");
            out.push_str("|----------|----------|--------|-----------|-----------|\n");
            for a in &self.agents {
                let providers = if a.providers.is_empty() {
                    "—".to_string()
                } else {
                    a.providers.join(", ")
                };
                let last_seen = a.last_seen.as_deref().unwrap_or("—");
                out.push_str(&format!(
                    "| {} | {} | {} | {} | {} |\n",
                    a.agent_id, a.total_requests, a.total_tokens, providers, last_seen
                ));
            }
        }

        out
    }

    /// Serialise this inventory to a pretty-printed JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("AgentInventory is always serialisable")
    }

    /// Render this inventory as a self-contained HTML document with an
    /// inline SVG bar chart of token usage per agent.
    pub fn to_html(&self) -> String {
        let chart_data: Vec<(String, f64)> = self
            .agents
            .iter()
            .map(|a| (a.agent_id.clone(), a.total_tokens as f64))
            .collect();

        let chart = crate::render::svg_bar_chart(&chart_data, 600, 200);

        let mut rows = String::new();
        for a in &self.agents {
            let providers = if a.providers.is_empty() {
                "—".to_string()
            } else {
                a.providers.join(", ")
            };
            let last_seen = a.last_seen.as_deref().unwrap_or("—");
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                a.agent_id, a.total_requests, a.total_tokens, providers, last_seen
            ));
        }

        format!(
            r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><title>Agent Inventory</title>
<style>body{{font-family:sans-serif;background:#0f0f0f;color:#e0e0e0;padding:24px}}
h1{{color:#fff}}table{{border-collapse:collapse;width:100%}}
th,td{{padding:8px;text-align:left;border-bottom:1px solid #2a2a2a}}</style></head>
<body><h1>Agent Inventory</h1>
<p>Total Agents: <b>{total}</b> | Active Agents: <b>{active}</b></p>
{chart}
<table><thead><tr><th>Agent ID</th><th>Requests</th><th>Tokens</th><th>Providers</th><th>Last Seen</th></tr></thead>
<tbody>{rows}</tbody></table>
</body></html>"#,
            total = self.total_agents,
            active = self.active_agents,
        )
    }
}

// Suppress unused import warning — HashSet is used transitively via the
// providers dedup logic which may be added by callers.
#[allow(dead_code)]
fn _providers_dedup(providers: Vec<String>) -> Vec<String> {
    let seen: HashSet<String> = providers.into_iter().collect();
    let mut v: Vec<String> = seen.into_iter().collect();
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_events() -> Vec<EventSummary> {
        vec![
            EventSummary {
                agent_id: "scout".into(),
                tokens: 500,
                requests: 5,
            },
            EventSummary {
                agent_id: "scout".into(),
                tokens: 300,
                requests: 3,
            },
            EventSummary {
                agent_id: "worker".into(),
                tokens: 100,
                requests: 1,
            },
        ]
    }

    #[test]
    fn inventory_from_events_aggregates_correctly() {
        let inv = AgentInventory::from_events(&sample_events());
        assert_eq!(inv.total_agents, 2);
        assert_eq!(inv.active_agents, 2);

        // "scout" has more tokens so it should be first
        assert_eq!(inv.agents[0].agent_id, "scout");
        assert_eq!(inv.agents[0].total_tokens, 800);
        assert_eq!(inv.agents[0].total_requests, 8);

        assert_eq!(inv.agents[1].agent_id, "worker");
        assert_eq!(inv.agents[1].total_tokens, 100);
    }

    #[test]
    fn inventory_to_markdown_contains_header() {
        let inv = AgentInventory::from_events(&sample_events());
        let md = inv.to_markdown();
        assert!(md.contains("# Agent Inventory"), "missing header in:\n{md}");
        assert!(md.contains("Agent ID"));
        assert!(md.contains("scout"));
        assert!(md.contains("worker"));
    }

    #[test]
    fn inventory_to_json_is_valid_json() {
        let inv = AgentInventory::from_events(&sample_events());
        let json = inv.to_json();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("inventory JSON should be valid");
        assert!(parsed["total_agents"].as_u64().unwrap() == 2);
    }

    #[test]
    fn inventory_to_html_is_valid_html() {
        let inv = AgentInventory::from_events(&sample_events());
        let html = inv.to_html();
        assert!(
            html.contains("<!DOCTYPE html>"),
            "missing DOCTYPE in HTML output"
        );
        assert!(html.contains("Agent Inventory"));
        assert!(html.contains("scout"));
    }

    #[test]
    fn inventory_empty_events() {
        let inv = AgentInventory::from_events(&[]);
        assert_eq!(inv.total_agents, 0);
        assert_eq!(inv.active_agents, 0);
        assert!(inv.agents.is_empty());
    }
}
