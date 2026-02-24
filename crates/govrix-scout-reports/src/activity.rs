//! Activity log — aggregate event data into hourly activity buckets.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::usage::EventSummary;

/// Cost per 1 000 tokens in USD used for activity-log cost estimates.
const COST_PER_1K_TOKENS: f64 = 0.002;

/// One entry in the activity log representing a single (agent, hour) bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    /// The agent identifier.
    pub agent_id: String,
    /// Total tokens consumed in this hour bucket.
    pub tokens: u64,
    /// Estimated cost in USD for this bucket.
    pub cost_usd: f64,
    /// Total proxy requests in this hour bucket.
    pub requests: u64,
    /// Hour bucket in `"YYYY-MM-DDTHH:00"` format.
    pub hour: String,
}

/// Workspace-level activity log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLog {
    /// All activity entries sorted by hour ascending then agent ascending.
    pub entries: Vec<ActivityEntry>,
    /// The hour bucket that had the most requests, if any entries exist.
    pub peak_hour: Option<String>,
    /// Number of requests in the peak hour.
    pub peak_requests: u64,
    /// Total error count (placeholder — populated by callers that have
    /// error metadata; always 0 when built from plain [`EventSummary`]s).
    pub error_count: u64,
}

impl ActivityLog {
    /// Build an [`ActivityLog`] from a slice of [`EventSummary`] records.
    ///
    /// Because [`EventSummary`] does not carry timestamp information, all
    /// events are assigned to the synthetic hour bucket `"unknown"`. Callers
    /// that have richer event data (e.g. from the database) should build
    /// [`ActivityEntry`] values directly and construct [`ActivityLog`] via
    /// [`ActivityLog::from_entries`].
    pub fn from_events(events: &[EventSummary]) -> Self {
        // (tokens, requests) keyed by (agent_id, hour)
        let mut map: HashMap<(String, String), (u64, u64)> = HashMap::new();

        for event in events {
            let key = (event.agent_id.clone(), "unknown".to_string());
            let entry = map.entry(key).or_default();
            entry.0 += event.tokens;
            entry.1 += event.requests;
        }

        let entries: Vec<ActivityEntry> = map
            .into_iter()
            .map(|((agent_id, hour), (tokens, requests))| ActivityEntry {
                agent_id,
                tokens,
                cost_usd: (tokens as f64 / 1000.0) * COST_PER_1K_TOKENS,
                requests,
                hour,
            })
            .collect();

        Self::from_entries(entries, 0)
    }

    /// Build an [`ActivityLog`] from pre-constructed entries.
    ///
    /// `error_count` is the total number of error events in the same window.
    pub fn from_entries(mut entries: Vec<ActivityEntry>, error_count: u64) -> Self {
        // Sort deterministically: hour asc, agent asc
        entries.sort_by(|a, b| a.hour.cmp(&b.hour).then(a.agent_id.cmp(&b.agent_id)));

        // Aggregate requests per hour to find the peak
        let mut requests_per_hour: HashMap<String, u64> = HashMap::new();
        for e in &entries {
            *requests_per_hour.entry(e.hour.clone()).or_default() += e.requests;
        }

        let (peak_hour, peak_requests) = requests_per_hour
            .into_iter()
            .max_by_key(|(_, r)| *r)
            .map(|(h, r)| (Some(h), r))
            .unwrap_or((None, 0));

        Self {
            entries,
            peak_hour,
            peak_requests,
            error_count,
        }
    }

    /// Render this log as a Markdown table.
    pub fn to_markdown(&self) -> String {
        let mut out = String::from("# Activity Log\n\n");

        if let Some(ref peak) = self.peak_hour {
            out.push_str(&format!(
                "Peak hour: **{}** ({} requests)  \n",
                peak, self.peak_requests
            ));
        }
        out.push_str(&format!("Errors: **{}**\n\n", self.error_count));

        if !self.entries.is_empty() {
            out.push_str("| Hour | Agent | Requests | Tokens | Cost (USD) |\n");
            out.push_str("|------|-------|----------|--------|------------|\n");
            for e in &self.entries {
                out.push_str(&format!(
                    "| {} | {} | {} | {} | ${:.6} |\n",
                    e.hour, e.agent_id, e.requests, e.tokens, e.cost_usd
                ));
            }
        }

        out
    }

    /// Serialise this log to a pretty-printed JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("ActivityLog is always serialisable")
    }

    /// Render this log as a self-contained HTML document with an inline SVG
    /// bar chart of requests per hour.
    pub fn to_html(&self) -> String {
        // Build per-hour aggregates for the chart
        let mut requests_per_hour: std::collections::BTreeMap<String, f64> =
            std::collections::BTreeMap::new();
        for e in &self.entries {
            *requests_per_hour.entry(e.hour.clone()).or_default() += e.requests as f64;
        }

        let chart_data: Vec<(String, f64)> = requests_per_hour.into_iter().collect();
        let chart = crate::render::svg_bar_chart(&chart_data, 600, 200);

        let mut rows = String::new();
        for e in &self.entries {
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>${:.6}</td></tr>",
                e.hour, e.agent_id, e.requests, e.tokens, e.cost_usd
            ));
        }

        let peak_str = match &self.peak_hour {
            Some(h) => format!("{} ({} requests)", h, self.peak_requests),
            None => "—".to_string(),
        };

        format!(
            r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><title>Activity Log</title>
<style>body{{font-family:sans-serif;background:#0f0f0f;color:#e0e0e0;padding:24px}}
h1{{color:#fff}}table{{border-collapse:collapse;width:100%}}
th,td{{padding:8px;text-align:left;border-bottom:1px solid #2a2a2a}}</style></head>
<body><h1>Activity Log</h1>
<p>Peak Hour: <b>{peak}</b> | Errors: <b>{errors}</b></p>
{chart}
<table><thead><tr><th>Hour</th><th>Agent</th><th>Requests</th><th>Tokens</th><th>Cost (USD)</th></tr></thead>
<tbody>{rows}</tbody></table>
</body></html>"#,
            peak = peak_str,
            errors = self.error_count,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_events() -> Vec<EventSummary> {
        vec![
            EventSummary {
                agent_id: "alpha".into(),
                tokens: 400,
                requests: 4,
            },
            EventSummary {
                agent_id: "beta".into(),
                tokens: 200,
                requests: 2,
            },
        ]
    }

    fn hourly_entries() -> Vec<ActivityEntry> {
        vec![
            ActivityEntry {
                agent_id: "alpha".into(),
                tokens: 400,
                cost_usd: 0.0008,
                requests: 10,
                hour: "2026-02-19T14:00".into(),
            },
            ActivityEntry {
                agent_id: "alpha".into(),
                tokens: 100,
                cost_usd: 0.0002,
                requests: 2,
                hour: "2026-02-19T15:00".into(),
            },
            ActivityEntry {
                agent_id: "beta".into(),
                tokens: 50,
                cost_usd: 0.0001,
                requests: 1,
                hour: "2026-02-19T15:00".into(),
            },
        ]
    }

    #[test]
    fn activity_log_from_events_builds() {
        let log = ActivityLog::from_events(&sample_events());
        // Both agents land in the "unknown" bucket
        assert_eq!(log.entries.len(), 2);
        assert_eq!(log.error_count, 0);
    }

    #[test]
    fn activity_log_identifies_peak_hour() {
        let log = ActivityLog::from_entries(hourly_entries(), 0);
        assert_eq!(log.peak_hour.as_deref(), Some("2026-02-19T14:00"));
        assert_eq!(log.peak_requests, 10);
    }

    #[test]
    fn activity_log_to_markdown_contains_header() {
        let log = ActivityLog::from_entries(hourly_entries(), 3);
        let md = log.to_markdown();
        assert!(md.contains("# Activity Log"), "missing header:\n{md}");
        assert!(md.contains("2026-02-19T14:00"));
        assert!(md.contains("Errors: **3**"));
    }

    #[test]
    fn activity_log_to_html_is_valid_html() {
        let log = ActivityLog::from_entries(hourly_entries(), 0);
        let html = log.to_html();
        assert!(
            html.contains("<!DOCTYPE html>"),
            "missing DOCTYPE in HTML output"
        );
        assert!(html.contains("Activity Log"));
        assert!(html.contains("2026-02-19T14:00"));
    }

    #[test]
    fn activity_log_to_json_is_valid_json() {
        let log = ActivityLog::from_events(&sample_events());
        let json = log.to_json();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("activity log JSON should be valid");
        assert!(parsed["entries"].is_array());
    }

    #[test]
    fn activity_log_empty_events() {
        let log = ActivityLog::from_events(&[]);
        assert!(log.entries.is_empty());
        assert!(log.peak_hour.is_none());
        assert_eq!(log.peak_requests, 0);
    }
}
