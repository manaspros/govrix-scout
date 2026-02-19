//! Report generation handlers (stub implementation for Phase 2).
//!
//! Route map:
//!   GET  /api/v1/reports/types    — list_types
//!   GET  /api/v1/reports          — list_reports
//!   POST /api/v1/reports/generate — generate_report

use axum::{http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

// ── Types ─────────────────────────────────────────────────────────────────────

/// Supported report types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ReportType {
    UsageSummary,
    CostBreakdown,
    AgentInventory,
    ActivityLog,
}

/// Request body for POST /api/v1/reports/generate
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct GenerateReportRequest {
    /// Report type to generate
    pub report_type: String,
    /// Optional ISO-8601 start of range
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    /// Optional ISO-8601 end of range
    pub to: Option<chrono::DateTime<chrono::Utc>>,
    /// Optional agent_id filter
    pub agent_id: Option<String>,
    /// Output format: json or pdf (default: json)
    pub format: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// List available report types.
///
/// GET /api/v1/reports/types
pub async fn list_types() -> impl IntoResponse {
    Json(json!({
        "data": [
            {
                "id": "usage_summary",
                "name": "Usage Summary",
                "description": "Aggregate request counts, token usage, and costs over a time period",
                "formats": ["json", "pdf"],
            },
            {
                "id": "cost_breakdown",
                "name": "Cost Breakdown",
                "description": "Detailed cost analysis by agent, model, or provider",
                "formats": ["json", "pdf"],
            },
            {
                "id": "agent_inventory",
                "name": "Agent Inventory",
                "description": "Full registry of all observed AI agents with statistics",
                "formats": ["json", "pdf"],
            },
            {
                "id": "activity_log",
                "name": "Activity Log",
                "description": "Chronological log of all intercepted events for an agent or session",
                "formats": ["json"],
            },
        ],
        "total": 4,
    }))
}

/// List previously generated reports.
///
/// GET /api/v1/reports
pub async fn list_reports() -> impl IntoResponse {
    // Phase 4 will persist report metadata. Return empty for now.
    Json(json!({
        "data": [],
        "total": 0,
        "note": "Report persistence is available in AgentMesh SaaS — see agentmesh.io",
    }))
}

/// Queue a report for generation and return a stub report ID.
///
/// POST /api/v1/reports/generate
pub async fn generate_report(Json(body): Json<GenerateReportRequest>) -> impl IntoResponse {
    // Validate report type
    let valid_types = [
        "usage_summary",
        "cost_breakdown",
        "agent_inventory",
        "activity_log",
    ];
    if !valid_types.contains(&body.report_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid report_type",
                "valid_types": valid_types,
            })),
        );
    }

    let report_id = Uuid::now_v7();
    let now = Utc::now();

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "data": {
                "report_id": report_id.to_string(),
                "status": "queued",
                "report_type": body.report_type,
                "format": body.format.unwrap_or_else(|| "json".to_string()),
                "created_at": now.to_rfc3339(),
                "note": "Full report generation with PDF export available in AgentMesh SaaS — see agentmesh.io",
            }
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_report_types() {
        let valid_types = [
            "usage_summary",
            "cost_breakdown",
            "agent_inventory",
            "activity_log",
        ];
        assert!(valid_types.contains(&"usage_summary"));
        assert!(valid_types.contains(&"cost_breakdown"));
        assert!(!valid_types.contains(&"invalid_type"));
    }

    #[test]
    fn generate_report_request_deserialization() {
        let json_str = r#"{"report_type": "usage_summary", "format": "pdf"}"#;
        let req: GenerateReportRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.report_type, "usage_summary");
        assert_eq!(req.format.as_deref(), Some("pdf"));
        assert!(req.from.is_none());
        assert!(req.agent_id.is_none());
    }
}
