//! Event list and detail handlers.
//!
//! Route map:
//!   GET  /api/v1/events                       — list_events
//!   GET  /api/v1/events/:id                   — get_event
//!   GET  /api/v1/events/sessions/:session_id  — get_session_events

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use govrix_scout_store::EventFilter;

use crate::api::state::AppState;

// ── Request types ─────────────────────────────────────────────────────────────

/// Query parameters for GET /api/v1/events
#[derive(Debug, Deserialize)]
pub struct ListEventsParams {
    /// Filter by agent_id
    pub agent_id: Option<String>,
    /// Filter by provider (openai, anthropic, etc.)
    pub provider: Option<String>,
    /// Filter by model name
    pub model: Option<String>,
    /// Filter by compliance_tag
    pub compliance_tag: Option<String>,
    /// ISO-8601 lower bound on timestamp (inclusive)
    pub from: Option<DateTime<Utc>>,
    /// ISO-8601 upper bound on timestamp (exclusive)
    pub to: Option<DateTime<Utc>>,
    /// Maximum number of results (default 50, max 500)
    pub limit: Option<i64>,
    /// Offset for pagination (default 0)
    pub offset: Option<i64>,
}

/// Paginated events response.
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct EventsResponse {
    pub data: Vec<serde_json::Value>,
    pub total: usize,
    pub limit: i64,
    pub offset: i64,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// List events with optional filters.
///
/// GET /api/v1/events
pub async fn list_events(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListEventsParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).clamp(1, 500);
    let offset = params.offset.unwrap_or(0).max(0);

    let filter = EventFilter {
        agent_id: params.agent_id,
        provider: params.provider,
        model: params.model,
        compliance_tag: params.compliance_tag,
        from: params.from,
        to: params.to,
        limit,
        offset,
        ..Default::default()
    };

    match govrix_scout_store::events::list_events(&state.pool, &filter).await {
        Ok(events) => {
            let total = events.len();
            (
                StatusCode::OK,
                Json(json!({
                    "data": events,
                    "total": total,
                    "limit": limit,
                    "offset": offset,
                })),
            )
        }
        Err(e) => {
            tracing::error!("list_events store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to query events", "detail": e.to_string() })),
            )
        }
    }
}

/// Get a single event by UUID.
///
/// GET /api/v1/events/:id
pub async fn get_event(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid event id — must be a UUID" })),
            );
        }
    };

    match govrix_scout_store::events::get_event(&state.pool, uuid).await {
        Ok(Some(event)) => (StatusCode::OK, Json(json!({ "data": event }))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "event not found", "id": id })),
        ),
        Err(e) => {
            tracing::error!("get_event store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch event", "detail": e.to_string() })),
            )
        }
    }
}

/// Get all events belonging to a session (ordered by timestamp ASC for audit trail).
///
/// GET /api/v1/events/sessions/:session_id
pub async fn get_session_events(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&session_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid session_id — must be a UUID" })),
            );
        }
    };

    match govrix_scout_store::events::get_session_events(&state.pool, uuid).await {
        Ok(events) => {
            let total = events.len();
            (
                StatusCode::OK,
                Json(json!({
                    "data": events,
                    "total": total,
                    "session_id": session_id,
                })),
            )
        }
        Err(e) => {
            tracing::error!("get_session_events store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch session events", "detail": e.to_string() })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_events_params_defaults() {
        // Simulate the default limit/offset logic used in the handler
        let params = ListEventsParams {
            agent_id: None,
            provider: None,
            model: None,
            compliance_tag: None,
            from: None,
            to: None,
            limit: None,
            offset: None,
        };
        let limit = params.limit.unwrap_or(50).clamp(1, 500);
        let offset = params.offset.unwrap_or(0).max(0);
        assert_eq!(limit, 50);
        assert_eq!(offset, 0);
    }

    #[test]
    fn list_events_params_clamps_limit() {
        let params = ListEventsParams {
            agent_id: None,
            provider: None,
            model: None,
            compliance_tag: None,
            from: None,
            to: None,
            limit: Some(9999),
            offset: Some(100),
        };
        let limit = params.limit.unwrap_or(50).clamp(1, 500);
        let offset = params.offset.unwrap_or(0).max(0);
        assert_eq!(limit, 500);
        assert_eq!(offset, 100);
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
