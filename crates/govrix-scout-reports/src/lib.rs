//! Govrix Scout Reports — HTML, JSON, and PDF report generation.
//!
//! Uses minijinja for HTML/text templating.
//! PDF generation (via printable HTML or a PDF crate) comes in Phase 4.
//!
//! Report types:
//! - `summary`     — overall agent activity summary
//! - `compliance`  — audit trail with lineage hash verification
//! - `cost`        — cost breakdown by agent, model, and day
//! - `pii`         — PII detection findings (type + location, no values)
//! - `inventory`   — workspace-level agent inventory catalogue
//! - `activity`    — hourly activity log with peak detection

pub mod activity;
pub mod cost;
pub mod inventory;
pub mod render;
pub mod usage;

pub use activity::ActivityLog;
pub use cost::CostBreakdown;
pub use inventory::AgentInventory;
pub use usage::UsageSummary;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Report output format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    Json,
    Html,
    /// PDF stub — requires Phase 4 implementation
    Pdf,
}

/// A generated report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Unique report ID (UUIDv7).
    pub id: uuid::Uuid,
    /// Report type name.
    pub report_type: String,
    /// The time range covered by this report.
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    /// Format this report was generated in.
    pub format: ReportFormat,
    /// The generated content (HTML string or JSON string).
    pub content: String,
    /// When this report was generated.
    pub generated_at: DateTime<Utc>,
}

/// Report generation context passed to templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportContext {
    pub title: String,
    pub from: String,
    pub to: String,
    pub generated_at: String,
    pub data: serde_json::Value,
    /// Upsell footer (required per build spec).
    pub upsell_url: String,
}

impl Default for ReportContext {
    fn default() -> Self {
        Self {
            title: String::new(),
            from: String::new(),
            to: String::new(),
            generated_at: Utc::now().to_rfc3339(),
            data: serde_json::Value::Null,
            upsell_url: "https://Govrix Scout.io".to_string(),
        }
    }
}

/// Aggregate output of all report types for a single event window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllReports {
    /// Overall usage roll-up.
    pub summary: UsageSummary,
    /// Agent inventory catalogue.
    pub inventory: AgentInventory,
    /// Hourly activity log.
    pub activity: ActivityLog,
    /// Cost breakdown by agent.
    pub cost: CostBreakdown,
}

/// Generate all report types from a single slice of [`usage::EventSummary`]
/// records.
///
/// # Example
/// ```
/// use govrix_scout_reports::{generate_all, usage::EventSummary};
/// let events = vec![EventSummary { agent_id: "bot".into(), tokens: 100, requests: 1 }];
/// let reports = generate_all(&events);
/// assert_eq!(reports.summary.unique_agents, 1);
/// assert_eq!(reports.inventory.total_agents, 1);
/// ```
pub fn generate_all(events: &[usage::EventSummary]) -> AllReports {
    AllReports {
        summary: UsageSummary::from_events(events),
        inventory: AgentInventory::from_events(events),
        activity: ActivityLog::from_events(events),
        cost: CostBreakdown::from_events(events),
    }
}

/// Generate a report from the given context using minijinja.
///
/// The template string is rendered with `context` as the template variables.
/// Returns the rendered string.
///
/// # Example
/// ```
/// use govrix_scout_reports::{ReportContext, render_template};
/// let ctx = ReportContext {
///     title: "Summary Report".to_string(),
///     ..Default::default()
/// };
/// let output = render_template("Hello {{ title }}!", &ctx).unwrap();
/// assert!(output.contains("Summary Report"));
/// ```
pub fn render_template(
    template_str: &str,
    context: &ReportContext,
) -> Result<String, minijinja::Error> {
    let mut env = minijinja::Environment::new();
    env.add_template("report", template_str)?;
    let tmpl = env.get_template("report")?;
    let ctx_value = minijinja::Value::from_serialize(context);
    tmpl.render(ctx_value)
}

/// Built-in summary report HTML template.
pub const SUMMARY_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>{{ title }}</title>
  <style>
    body { font-family: system-ui, sans-serif; max-width: 900px; margin: 2rem auto; }
    footer { margin-top: 3rem; font-size: 0.8rem; color: #666; }
  </style>
</head>
<body>
  <h1>{{ title }}</h1>
  <p>Period: {{ from }} — {{ to }}</p>
  <p>Generated: {{ generated_at }}</p>
  <hr>
  <pre>{{ data }}</pre>
  <footer>
    Powered by Govrix Scout OSS.
    <a href="{{ upsell_url }}">Upgrade to Govrix Scout Enterprise</a>
    for compliance policy enforcement, unlimited retention, and A2A identity.
  </footer>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::EventSummary;

    #[test]
    fn render_summary_template() {
        let ctx = ReportContext {
            title: "Test Report".to_string(),
            from: "2026-01-01".to_string(),
            to: "2026-01-07".to_string(),
            ..Default::default()
        };
        let rendered = render_template(SUMMARY_TEMPLATE, &ctx).expect("render failed");
        assert!(rendered.contains("Test Report"));
        assert!(rendered.contains("Govrix Scout.io"));
    }

    fn sample_events() -> Vec<EventSummary> {
        vec![
            EventSummary {
                agent_id: "agent-a".into(),
                tokens: 800,
                requests: 8,
            },
            EventSummary {
                agent_id: "agent-b".into(),
                tokens: 200,
                requests: 2,
            },
        ]
    }

    #[test]
    fn generate_all_produces_consistent_reports() {
        let events = sample_events();
        let reports = generate_all(&events);
        // All sub-reports should see the same two agents
        assert_eq!(reports.summary.unique_agents, 2);
        assert_eq!(reports.inventory.total_agents, 2);
        assert_eq!(reports.inventory.active_agents, 2);
        assert_eq!(reports.cost.by_agent.len(), 2);
        // Activity log: both agents land in "unknown" bucket
        assert_eq!(reports.activity.entries.len(), 2);
    }

    #[test]
    fn generate_all_empty_events() {
        let reports = generate_all(&[]);
        assert_eq!(reports.summary.unique_agents, 0);
        assert_eq!(reports.inventory.total_agents, 0);
        assert!(reports.activity.entries.is_empty());
        assert_eq!(reports.cost.total_tokens, 0);
    }
}
