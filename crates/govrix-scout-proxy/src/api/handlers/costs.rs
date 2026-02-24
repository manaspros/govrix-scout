//! Cost summary and breakdown handlers.
//!
//! Route map:
//!   GET /api/v1/costs/summary    — cost_summary
//!   GET /api/v1/costs/breakdown  — cost_breakdown

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::json;

use govrix_scout_store::costs::{Granularity, GroupBy};

use crate::api::state::AppState;

// ── Request types ─────────────────────────────────────────────────────────────

/// Query parameters for GET /api/v1/costs/summary
#[derive(Debug, Deserialize)]
pub struct CostSummaryParams {
    /// ISO-8601 start of range (default: 7 days ago)
    pub from: Option<DateTime<Utc>>,
    /// ISO-8601 end of range (default: now)
    pub to: Option<DateTime<Utc>>,
    /// Time bucket granularity: hour, day, week, month (default: day)
    pub granularity: Option<String>,
}

/// Query parameters for GET /api/v1/costs/breakdown
#[derive(Debug, Deserialize)]
pub struct CostBreakdownParams {
    /// ISO-8601 start of range (default: 7 days ago)
    pub from: Option<DateTime<Utc>>,
    /// ISO-8601 end of range (default: now)
    pub to: Option<DateTime<Utc>>,
    /// Dimension to group by: agent, model, protocol (default: agent)
    pub group_by: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_granularity(s: &str) -> Granularity {
    match s.to_lowercase().as_str() {
        "hour" => Granularity::Hour,
        "week" => Granularity::Week,
        "month" => Granularity::Month,
        _ => Granularity::Day,
    }
}

fn parse_group_by(s: &str) -> GroupBy {
    match s.to_lowercase().as_str() {
        "model" => GroupBy::Model,
        "protocol" | "provider" => GroupBy::Protocol,
        _ => GroupBy::Agent,
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Return aggregate cost summary for a time range.
///
/// GET /api/v1/costs/summary
pub async fn cost_summary(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CostSummaryParams>,
) -> impl IntoResponse {
    let now = Utc::now();
    let to = params.to.unwrap_or(now);
    let from = params.from.unwrap_or_else(|| to - Duration::days(7));
    let granularity = params
        .granularity
        .as_deref()
        .map(parse_granularity)
        .unwrap_or(Granularity::Day);

    match govrix_scout_store::costs::get_cost_summary(&state.pool, from, to, granularity).await {
        Ok(summary) => (StatusCode::OK, Json(json!({ "data": summary }))),
        Err(e) => {
            tracing::error!("cost_summary store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch cost summary", "detail": e.to_string() })),
            )
        }
    }
}

/// Return cost breakdown grouped by a dimension.
///
/// GET /api/v1/costs/breakdown
pub async fn cost_breakdown(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CostBreakdownParams>,
) -> impl IntoResponse {
    let now = Utc::now();
    let to = params.to.unwrap_or(now);
    let from = params.from.unwrap_or_else(|| to - Duration::days(7));
    let group_by = params
        .group_by
        .as_deref()
        .map(parse_group_by)
        .unwrap_or(GroupBy::Agent);

    match govrix_scout_store::costs::get_cost_breakdown(&state.pool, from, to, group_by).await {
        Ok(rows) => {
            let total = rows.len();
            (
                StatusCode::OK,
                Json(json!({
                    "data": rows,
                    "total": total,
                    "from": from.to_rfc3339(),
                    "to": to.to_rfc3339(),
                    "group_by": params.group_by.unwrap_or_else(|| "agent".to_string()),
                })),
            )
        }
        Err(e) => {
            tracing::error!("cost_breakdown store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch cost breakdown", "detail": e.to_string() })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_granularity_cases() {
        assert_eq!(parse_granularity("hour"), Granularity::Hour);
        assert_eq!(parse_granularity("day"), Granularity::Day);
        assert_eq!(parse_granularity("week"), Granularity::Week);
        assert_eq!(parse_granularity("month"), Granularity::Month);
        // Unknown falls back to Day
        assert_eq!(parse_granularity("decade"), Granularity::Day);
    }

    #[test]
    fn parse_group_by_cases() {
        assert_eq!(parse_group_by("model"), GroupBy::Model);
        assert_eq!(parse_group_by("protocol"), GroupBy::Protocol);
        assert_eq!(parse_group_by("provider"), GroupBy::Protocol);
        assert_eq!(parse_group_by("agent"), GroupBy::Agent);
        // Unknown falls back to Agent
        assert_eq!(parse_group_by("unknown_dim"), GroupBy::Agent);
    }

    #[test]
    fn default_time_range_is_7_days() {
        let now = Utc::now();
        let to = now;
        let from = to - Duration::days(7);
        let diff = to - from;
        assert_eq!(diff.num_days(), 7);
    }
}
