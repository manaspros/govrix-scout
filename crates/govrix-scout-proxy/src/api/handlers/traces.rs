//! Trace list and detail handlers.
//!
//! Route map:
//!   GET  /api/v1/traces            — list_traces
//!   GET  /api/v1/traces/:trace_id  — get_trace

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::api::state::AppState;

// ── Request types ─────────────────────────────────────────────────────────────

/// Query parameters for GET /api/v1/traces
#[derive(Debug, Deserialize)]
pub struct ListTracesParams {
    /// Filter by status (running, completed, stopped, failed)
    pub status: Option<String>,
    /// Filter by root_agent_id
    pub agent_id: Option<String>,
    /// Maximum number of results (default 50, max 500)
    pub limit: Option<i64>,
    /// Offset for pagination (default 0)
    pub offset: Option<i64>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// List traces with optional filters, ordered by started_at descending.
///
/// GET /api/v1/traces
pub async fn list_traces(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListTracesParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).clamp(1, 500);
    let offset = params.offset.unwrap_or(0).max(0);

    match govrix_scout_store::traces::list_traces(
        &state.pool,
        params.agent_id.as_deref(),
        params.status.as_deref(),
        limit,
        offset,
    )
    .await
    {
        Ok(traces) => {
            let total = traces.len();
            (
                StatusCode::OK,
                Json(json!({
                    "data": traces,
                    "total": total,
                    "limit": limit,
                    "offset": offset,
                })),
            )
        }
        Err(e) => {
            tracing::error!("list_traces store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to query traces", "detail": e.to_string() })),
            )
        }
    }
}

/// Get a single trace by UUID, including its spans (events).
///
/// GET /api/v1/traces/:trace_id
pub async fn get_trace(
    State(state): State<Arc<AppState>>,
    Path(trace_id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&trace_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid trace_id — must be a UUID" })),
            );
        }
    };

    // Fetch the trace record
    let trace = match govrix_scout_store::traces::get_trace(&state.pool, uuid).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "trace not found", "trace_id": trace_id })),
            );
        }
        Err(e) => {
            tracing::error!("get_trace store error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch trace", "detail": e.to_string() })),
            );
        }
    };

    // Fetch spans (events belonging to this trace)
    let spans = match govrix_scout_store::traces::get_trace_spans(&state.pool, uuid).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("get_trace_spans store error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch trace spans", "detail": e.to_string() })),
            );
        }
    };

    // Merge the trace object with spans
    let mut data = trace;
    if let Some(obj) = data.as_object_mut() {
        obj.insert("spans".to_string(), json!(spans));
    }

    (StatusCode::OK, Json(json!({ "data": data })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_traces_params_defaults() {
        let params = ListTracesParams {
            status: None,
            agent_id: None,
            limit: None,
            offset: None,
        };
        let limit = params.limit.unwrap_or(50).clamp(1, 500);
        let offset = params.offset.unwrap_or(0).max(0);
        assert_eq!(limit, 50);
        assert_eq!(offset, 0);
    }

    #[test]
    fn list_traces_params_clamps_limit() {
        let params = ListTracesParams {
            status: None,
            agent_id: None,
            limit: Some(9999),
            offset: Some(10),
        };
        let limit = params.limit.unwrap_or(50).clamp(1, 500);
        let offset = params.offset.unwrap_or(0).max(0);
        assert_eq!(limit, 500);
        assert_eq!(offset, 10);
    }

    #[test]
    fn uuid_parse_valid() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        assert!(Uuid::parse_str(id).is_ok());
    }

    #[test]
    fn uuid_parse_invalid() {
        let id = "not-a-uuid";
        assert!(Uuid::parse_str(id).is_err());
    }
}
